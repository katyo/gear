use super::{
    DetectOpts, FileKind, FormatArgs, LdScript, PlatformKind, SemVer, SizeInfo, ToolchainOpts,
};
use crate::{
    qjs,
    system::{check_access, exec_out, write_file, AccessMode, Path, PathBuf},
    Actual, Artifact, ArtifactStore, DataHasher, Directory, Error, Input, Mut, Output, Ref, Result,
    Rule, RuleApi, Set, WeakArtifact,
};
use futures::future::join_all;
use std::{future::Future, iter::once, pin::Pin, result::Result as StdResult, str::FromStr};

macro_rules! log_out {
    ($res:ident) => {
        log_out!(@out $res);
        log_out!(@err $res);
    };

    (@out $res:ident) => {
        if !$res.out.is_empty() {
            for line in $res.out.split('\n') {
                log::warn!("{}", line);
            }
        }
    };

    (@err $res:ident) => {
        if !$res.err.is_empty() {
            for line in $res.err.split('\n') {
                log::error!("{}", line);
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
#[quickjs(untagged, rename_all = "lowercase")]
pub enum CompilerKind {
    Gcc,
    Llvm,
}

impl FromStr for CompilerKind {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self> {
        if name.ends_with("gcc") || name.ends_with("g++") {
            Ok(Self::Gcc)
        } else if name.ends_with("clang") || name.ends_with("clang++") {
            Ok(Self::Llvm)
        } else {
            Err(format!("Unsupported compiler: {}", name).into())
        }
    }
}

struct PropsInternal {
    cc: String,
    ar: String,
    nm: String,
    size: String,
    strip: String,
    objcopy: String,
    objdump: String,
    readelf: String,

    kind: CompilerKind,
    version: String,
    target: String,
    platform: PlatformKind,
}

impl PropsInternal {
    async fn new(opts: DetectOpts) -> Result<Self> {
        let path = &opts.compiler;
        let kind = CompilerKind::from_str(path)?;

        let (version, target, ar, nm, size, strip, objcopy, objdump, readelf) = match kind {
            CompilerKind::Gcc => {
                let mut version = exec_out(path, &["-dumpversion"]).await?.success()?.out;
                version.retain(|c| c != '\n');

                log::debug!("gcc version: {}", version);

                let mut target = exec_out(path, &["-dumpmachine"]).await?.success()?.out;
                target.retain(|c| c != '\n');

                log::debug!("gcc target: {}", target);

                let pre_path = path.strip_suffix("gcc").ok_or_else(|| {
                    format!(
                        "Invalid GCC compiler path: `{}`. It should ends with gcc",
                        path
                    )
                })?;

                let ar = format!("{}-ar", path);
                let nm = format!("{}-nm", path);
                let size = format!("{}size", pre_path);
                let strip = format!("{}strip", pre_path);
                let objcopy = format!("{}objcopy", pre_path);
                let objdump = format!("{}objdump", pre_path);
                let readelf = format!("{}readelf", pre_path);

                (
                    version, target, ar, nm, size, strip, objcopy, objdump, readelf,
                )
            }
            CompilerKind::Llvm => {
                let target = if opts.target.is_empty() {
                    env!("BUILD_TARGET")
                } else {
                    &opts.target
                };

                let data = exec_out(path, &["-target", target, "--version"])
                    .await?
                    .success()?
                    .out;

                let mut lines = data.split('\n');

                let version = lines
                    .next()
                    .and_then(|line| {
                        let mut chunks = line.split("clang version ");
                        chunks
                            .next()
                            .and_then(|_| chunks.next())
                            .and_then(|chunk| chunk.split(' ').next())
                    })
                    .ok_or_else(|| format!("Unable to determine clang version"))?
                    .into();

                log::debug!("clang version: {}", version);

                let target = lines
                    .next()
                    .and_then(|line| {
                        if line.starts_with("Target:") {
                            Some(line[7..].trim().into())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| format!("Unable to determine clang target"))?;

                log::debug!("clang target: {}", target);

                async fn find_tool(path: &str, name: &str) -> Result<String> {
                    let mut path = exec_out(path, &["--print-prog-name", name])
                        .await?
                        .success()?
                        .out;
                    path.retain(|c| c != '\n');
                    Ok(path)
                }

                let mut paths = join_all(
                    [
                        "llvm-ar",
                        "llvm-nm",
                        "llvm-size",
                        "llvm-strip",
                        "llvm-objcopy",
                        "llvm-objdump",
                        "llvm-readelf",
                    ]
                    .iter()
                    .map(|name| find_tool(path, name)),
                )
                .await
                .into_iter()
                .collect::<Result<Vec<_>>>()?
                .into_iter();

                let ar = paths.next().unwrap();
                let nm = paths.next().unwrap();
                let size = paths.next().unwrap();
                let strip = paths.next().unwrap();
                let objcopy = paths.next().unwrap();
                let objdump = paths.next().unwrap();
                let readelf = paths.next().unwrap();

                (
                    version, target, ar, nm, size, strip, objcopy, objdump, readelf,
                )
            }
        };

        join_all(
            [&ar, &nm, &size, &strip, &objcopy, &objdump, &readelf]
                .iter()
                .map(|path| check_access(path, AccessMode::EXECUTE)),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

        let platform = PlatformKind::from_target(&target)?;

        Ok(Self {
            cc: opts.compiler.clone(),
            ar,
            nm,
            size,
            strip,
            objcopy,
            objdump,
            readelf,

            kind,
            version,
            target,
            platform,
        })
    }
}

#[derive(Clone)]
pub struct CompilerConfig(Ref<Internal>);

#[derive(Clone)]
pub(self) struct Internal {
    props: Ref<PropsInternal>,
    opts: ToolchainOpts,
    compile_opts: Vec<String>,
    compile_hash: String,
    link_opts: Vec<String>,
    dump_opts: Vec<String>,
    strip_opts: Vec<String>,
}

impl Internal {
    pub async fn detect(opts: DetectOpts) -> Result<Self> {
        let props = PropsInternal::new(opts).await?;

        let compile_opts = Vec::<String>::default();
        let compile_hash = DataHasher::hash_base64_string(&compile_opts);
        let link_opts = Default::default();
        let dump_opts = Default::default();
        let strip_opts = Default::default();

        Ok(Self {
            props: Ref::new(props),
            opts: Default::default(),
            compile_opts,
            compile_hash,
            link_opts,
            dump_opts,
            strip_opts,
        })
    }

    pub fn config(&self, new_opts: ToolchainOpts) -> Self {
        let mut opts = self.opts.clone();
        opts.extend(Some(new_opts));

        let mut compile_opts = Vec::default();
        if self.props.kind == CompilerKind::Llvm {
            compile_opts.push("-target".into());
            compile_opts.push(self.props.target.clone());
        }
        opts.common.fmt_args(&mut compile_opts);
        opts.compile.fmt_args(&mut compile_opts);

        let compile_hash = DataHasher::hash_base64_string(&compile_opts);

        let mut link_opts = Vec::default();
        if self.props.kind == CompilerKind::Llvm {
            link_opts.push("-target".into());
            link_opts.push(self.props.target.clone());
        }
        opts.common.fmt_args(&mut link_opts);
        opts.link.fmt_args(&mut link_opts);

        let mut dump_opts = Vec::default();
        opts.dump.fmt_args(&mut dump_opts);

        let mut strip_opts = Vec::default();
        opts.strip.fmt_args(&mut strip_opts);

        Self {
            props: self.props.clone(),
            opts,
            compile_opts,
            compile_hash,
            link_opts,
            dump_opts,
            strip_opts,
        }
    }
}

pub(self) struct CompileInternal {
    cfg: CompilerConfig,
    store: ArtifactStore,
    args: Vec<String>,
    src: Artifact<Input, Actual>,
    dep: PathBuf,
    incs: Mut<Set<Artifact<Input, Actual>>>,
    dst: WeakArtifact<Output, Actual>,
}

impl Drop for CompileInternal {
    fn drop(&mut self) {
        log::debug!("Compile::drop");
    }
}

impl RuleApi for CompileInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        once(&self.src)
            .chain(self.incs.read().iter())
            .map(|input| input.clone().into_kind_any())
            .collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.dst
            .try_ref()
            .map(|input| input.into_kind_any())
            .into_iter()
            .collect()
    }

    fn invoke(self: Ref<Self>) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            log::debug!("Compile::invoke");
            if self.dst.try_ref().is_some() {
                let res = exec_out(&self.cfg.0.props.cc, &self.args).await?;
                log_out!(res);
                res.success()?;

                let dep_path = &self.dep;
                if dep_path.is_file().await {
                    let src_name = self.src.name();
                    // reload generated deps
                    let incs = self
                        .store
                        .read_deps(dep_path, |src| src != src_name)
                        .await?;
                    *self.incs.write() = incs;
                }
            }
            Ok(())
        })
    }
}

pub(self) struct LinkInternal {
    cfg: CompilerConfig,
    link: bool, // link or archive
    args: Vec<String>,
    objs: Set<Artifact<Input, Actual>>,
    script: Option<Artifact<Input, Actual>>,
    out: WeakArtifact<Output, Actual>,
}

impl Drop for LinkInternal {
    fn drop(&mut self) {
        log::debug!("Link::drop");
    }
}

impl RuleApi for LinkInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.script
            .iter()
            .chain(self.objs.iter())
            .map(|input| input.clone().into_kind_any())
            .collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.out
            .try_ref()
            .map(|input| input.into_kind_any())
            .into_iter()
            .collect()
    }

    fn invoke(self: Ref<Self>) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            log::debug!("Link::invoke");
            if self.out.try_ref().is_some() {
                let cmd = if self.link {
                    &self.cfg.0.props.cc
                } else {
                    &self.cfg.0.props.ar
                };
                let res = exec_out(cmd, &self.args).await?;
                log_out!(res);
                res.success()?;
            }
            Ok(())
        })
    }
}

pub(self) struct StripInternal {
    cfg: CompilerConfig,
    args: Vec<String>,
    obj: Artifact<Input, Actual>,
    out: WeakArtifact<Output, Actual>,
    info: WeakArtifact<Output, Actual>,
}

impl Drop for StripInternal {
    fn drop(&mut self) {
        log::debug!("Strip::drop");
    }
}

impl RuleApi for StripInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        vec![self.obj.clone().into_kind_any()]
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.out
            .try_ref()
            .map(|input| input.into_kind_any())
            .into_iter()
            .chain(
                self.info
                    .try_ref()
                    .map(|input| input.into_kind_any())
                    .into_iter(),
            )
            .collect()
    }

    fn invoke(self: Ref<Self>) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            log::debug!("Strip::invoke");
            if self.out.try_ref().is_some() {
                let res = exec_out(&self.cfg.0.props.strip, &self.args).await?;
                log_out!(res);
                res.success()?;
            }
            Ok(())
        })
    }
}

enum CInputKind {
    C,
    Cxx,
    Asm,
}

impl FromStr for CInputKind {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s {
            "c" => Ok(Self::C),
            "cpp" | "cxx" | "c++" => Ok(Self::Cxx),
            "S" | "s" | "asm" => Ok(Self::Asm),
            _ => Err(()),
        }
    }
}

impl CInputKind {
    pub fn from_name(name: impl AsRef<str>) -> Result<Self> {
        let name = name.as_ref();
        let ext = name
            .rsplit('.')
            .next()
            .ok_or_else(|| format!("Unable to determine extension of source file `{}`", name))?;
        let kind =
            Self::from_str(ext).map_err(|_| format!("Unknown source file extension `{}`", ext))?;
        Ok(kind)
    }
}

enum COutputKind {
    Cpp,
    Asm,
    Obj,
}

impl AsRef<str> for COutputKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Cpp => "cpp",
            Self::Asm => "s",
            Self::Obj => "o",
        }
    }
}

impl COutputKind {
    pub fn make_extension<'a>(&'a self, name: &'a str) -> &'a str {
        if let Self::Cpp = self {
            name.rsplit('.').next().unwrap_or(self.as_ref())
        } else {
            self.as_ref()
        }
    }
}

impl CompilerConfig {
    async fn cc_raw(
        self,
        outdir: Directory,
        src: Artifact<Input, Actual>,
        outkind: COutputKind,
    ) -> Result<Artifact<Input, Actual>> {
        let cfg = self;
        let src_name = src.name().clone();

        let inkind = CInputKind::from_name(&src_name)?;
        let cfg_hash = cfg.0.compile_hash.clone();

        let dst_ext = outkind.make_extension(&src_name);
        let dst_name = format!("{}.{}.{}", src_name, cfg_hash, dst_ext);
        let dst = outdir.output(dst_name).await?;
        let dst_name = dst.name().clone();

        let dep_name = format!("{}.dep", dst_name);
        let dep_path = PathBuf::from(&dep_name);

        let store: &ArtifactStore = outdir.as_ref();
        let incs = if dep_path.is_file().await {
            // preload already generated deps
            store.read_deps(&dep_path, |src| src != &src_name).await?
        } else {
            // deps will be generated under compilation
            Default::default()
        };

        let inopt = match inkind {
            CInputKind::C => "-xc",
            CInputKind::Cxx => "-xc++",
            CInputKind::Asm => "-xassembler",
        };

        let outopt = match outkind {
            COutputKind::Cpp => "-E",
            COutputKind::Asm => "-S",
            COutputKind::Obj => "-c",
        };

        let mut args = vec![
            inopt.into(),
            outopt.into(),
            "-MMD".into(),
            "-MF".into(),
            dep_name,
            "-o".into(),
            dst_name,
            src_name,
        ];
        args.extend(cfg.0.compile_opts.iter().cloned());

        log::debug!("Compile::new");

        let rule = Ref::new(CompileInternal {
            cfg,
            store: store.clone(),
            args,
            src,
            dep: dep_path,
            incs: Mut::new(incs),
            dst: dst.weak(),
        });

        dst.set_rule(Rule::from_api(rule));

        Ok(dst.into())
    }

    async fn ld_raw(
        self,
        outdir: Directory,
        name: impl AsRef<str>,
        kind: FileKind,
        objs: Set<Artifact<Input, Actual>>,
        script: Option<Artifact<Input, Actual>>,
    ) -> Result<Artifact<Input, Actual>> {
        let cfg = self;
        let objs_names = objs.iter().map(|obj| obj.name()).cloned();

        let out_name = kind.file_name(&cfg.0.props.platform, name);
        let out = outdir.output(out_name).await?;
        let out_name = out.name().clone();

        let link = !matches!(kind, FileKind::Static { .. });

        let map_name = format!("{}.map", out_name);
        let mut args = vec![if link { "-o" } else { "cr" }.into(), out_name];
        args.extend(objs_names);
        if link {
            if matches!(kind, FileKind::Dynamic { .. }) {
                args.push("-shared".into());
            }
            args.extend(cfg.0.link_opts.iter().cloned());
            if let Some(script) = &script {
                args.push(format!("-T{}", script.name()));
            }
            args.push(format!("-Wl,-Map,{}", map_name));
        }

        log::debug!("Link::new");

        let rule = Ref::new(LinkInternal {
            cfg,
            link,
            args,
            objs,
            script,
            out: out.weak(),
        });

        out.set_rule(Rule::from_api(rule));

        Ok(out.into())
    }

    async fn strip_raw(
        self,
        outdir: Directory,
        obj: Artifact<Input, Actual>,
    ) -> Result<(Artifact<Input, Actual>, Artifact<Input, Actual>)> {
        let cfg = self;
        let obj_name = obj.name().clone();

        let out_name = Path::new(&obj_name)
            .file_name()
            .ok_or_else(|| format!("Unable to determine file name to strip `{}`", obj_name))?
            .to_str()
            .unwrap();
        let out = outdir.output(out_name).await?;
        let out_name = out.name().clone();
        let info_name = format!("{}.strip", out_name);
        let info = outdir.output(info_name).await?;
        let info_name = info.name().clone();

        let mut args = Vec::default();
        args.extend(cfg.0.strip_opts.iter().cloned());
        args.push("-o".into());
        args.push(info_name);
        args.push(out_name);

        log::debug!("Strip::new");

        let rule = Ref::new(StripInternal {
            cfg,
            args,
            obj,
            out: out.weak(),
            info: info.weak(),
        });

        out.set_rule(Rule::from_api(rule.clone()));
        info.set_rule(Rule::from_api(rule));

        Ok((out.into(), info.into()))
    }
}

pub(super) struct LdScriptInternal {
    data: LdScript,
    out: WeakArtifact<Output, Actual>,
    incs: Set<Artifact<Input, Actual>>,
}

impl Drop for LdScriptInternal {
    fn drop(&mut self) {
        log::debug!("LdScript::drop");
    }
}

impl RuleApi for LdScriptInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.incs
            .iter()
            .map(|input| input.clone().into_kind_any())
            .collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.out
            .try_ref()
            .map(|input| input.into_kind_any())
            .into_iter()
            .collect()
    }

    fn invoke(self: Ref<Self>) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            log::debug!("LdScript::invoke");

            if let Some(out) = self.out.try_ref() {
                let mut data = LdScript::default();
                data.includes = self.incs.iter().map(|inc| inc.name().into()).collect();
                let content = format!("{}{}", self.data, data);
                write_file(out.name(), content).await?;
            }

            Ok(())
        })
    }
}

impl LdScriptInternal {
    pub async fn create(
        outdir: Directory,
        name: impl AsRef<str>,
        data: LdScript,
        incs: Set<Artifact<Input, Actual>>,
    ) -> Result<Artifact<Input, Actual>> {
        log::debug!("LdScript::new");

        let out_name = format!("{}.ld", name.as_ref());
        let out = outdir.output(out_name).await?;

        let rule = Ref::new(LdScriptInternal {
            data,
            out: out.weak(),
            incs,
        });

        out.set_rule(Rule::from_api(rule));

        Ok(out.into())
    }
}

#[derive(Debug, Clone, Copy, Default, qjs::FromJs)]
pub struct ArOptions {
    #[quickjs(default = "library_default")]
    library: bool,
}

#[derive(Debug, Clone, Default, qjs::FromJs)]
pub struct LdOptions {
    #[quickjs(default)]
    dynamic: bool,
    #[quickjs(default = "library_default")]
    library: bool,
    version: Option<SemVer>,
    script: Option<Artifact<Input, Actual>>,
}

const fn library_default() -> bool {
    true
}

#[derive(Debug, Clone, Default, qjs::FromJs)]
pub struct NmOptions {
    /// Only debug symbols
    #[quickjs(default)]
    debug: bool,
    /// Demangle symbols
    #[quickjs(default)]
    demangle: Option<String>,
    /// Dynamic symbols
    #[quickjs(default)]
    dynamic: bool,
    /// Only defined symbols
    #[quickjs(default)]
    defined: bool,
    /// Only undefined symbols
    #[quickjs(default)]
    undefined: bool,
    /// Only external symbols
    #[quickjs(default)]
    external: bool,
    /// Also special symbols
    #[quickjs(default)]
    special: bool,
    /// Also synthetic symbols
    #[quickjs(default)]
    synthetic: bool,
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(rename = "Compiler", cloneable)]
    impl CompilerConfig {
        pub fn new() -> Self {
            unimplemented!();
        }

        pub async fn detect(opts: qjs::Opt<DetectOpts>) -> Result<Self> {
            let opts = opts.0.unwrap_or_default().detect().await?;
            let intern = Internal::detect(opts).await?;
            Ok(Self(Ref::new(intern)))
        }

        pub fn config(&self, opts: qjs::Opt<ToolchainOpts>) -> Result<Self> {
            let opts = opts.0.unwrap_or_default();
            let intern = self.0.config(opts);
            Ok(Self(Ref::new(intern)))
        }

        #[quickjs(rename = "ccPath", get, enumerable)]
        pub fn cc_path(&self) -> &String {
            &self.0.props.cc
        }

        #[quickjs(rename = "arPath", get, enumerable)]
        pub fn ar_path(&self) -> &String {
            &self.0.props.ar
        }

        #[quickjs(rename = "nmPath", get, enumerable)]
        pub fn nm_path(&self) -> &String {
            &self.0.props.nm
        }

        #[quickjs(get, enumerable)]
        pub fn kind(&self) -> CompilerKind {
            self.0.props.kind
        }

        #[quickjs(get, enumerable)]
        pub fn version(&self) -> &String {
            &self.0.props.version
        }

        #[quickjs(get, enumerable)]
        pub fn target(&self) -> &String {
            &self.0.props.target
        }

        #[quickjs(get, enumerable)]
        pub async fn sysroot(self) -> Result<String> {
            Ok(exec_out(&self.0.props.cc, &["-print-sysroot"])
                .await?
                .success()?
                .out
                .trim()
                .into())
        }

        #[quickjs(get, enumerable)]
        pub async fn search_dirs(self) -> Result<Vec<String>> {
            Ok(exec_out(&self.0.props.cc, &["-print-search-dirs"])
                .await?
                .success()?
                .out
                .trim()
                .split(':')
                .map(|path| path.into())
                .collect())
        }

        /// Compile with object output
        pub async fn cc(
            self,
            outdir: Directory,
            src: Artifact<Input, Actual>,
        ) -> Result<Artifact<Input, Actual>> {
            self.cc_raw(outdir, src, COutputKind::Obj).await
        }

        /// Preprocess only without compilation
        pub async fn cpp(
            self,
            outdir: Directory,
            src: Artifact<Input, Actual>,
        ) -> Result<Artifact<Input, Actual>> {
            self.cc_raw(outdir, src, COutputKind::Cpp).await
        }

        /// Compile with assembler output
        pub async fn asm(
            self,
            outdir: Directory,
            src: Artifact<Input, Actual>,
        ) -> Result<Artifact<Input, Actual>> {
            self.cc_raw(outdir, src, COutputKind::Asm).await
        }

        /// Archive objects to static library
        pub async fn ar(
            self,
            outdir: Directory,
            name: String,
            objs: Set<Artifact<Input, Actual>>,
            opts: qjs::Opt<ArOptions>,
        ) -> Result<Artifact<Input, Actual>> {
            let ArOptions { library } = opts.0.unwrap_or_default();
            self.ld_raw(outdir, name, FileKind::Static { library }, objs, None)
                .await
        }

        /// Link objects
        pub async fn ld(
            self,
            outdir: Directory,
            name: String,
            objs: Set<Artifact<Input, Actual>>,
            opts: qjs::Opt<LdOptions>,
        ) -> Result<Artifact<Input, Actual>> {
            let LdOptions {
                dynamic,
                library,
                version,
                script,
            } = opts.0.unwrap_or_default();
            self.ld_raw(
                outdir,
                name,
                if dynamic {
                    FileKind::Dynamic { library, version }
                } else {
                    FileKind::Executable
                },
                objs,
                script,
            )
            .await
        }

        /// Strip objects
        pub async fn strip(
            self,
            outdir: Directory,
            obj: Artifact<Input, Actual>,
        ) -> Result<(Artifact<Input, Actual>, Artifact<Input, Actual>)> {
            self.strip_raw(outdir, obj).await
        }

        /// Measure size
        pub async fn size(self, file: String, files: qjs::Rest<String>) -> Result<SizeInfo> {
            let mut args = vec!["--format=SysV".into(), file];
            args.extend(files.0);
            let res = exec_out(&self.0.props.size, &args).await?;
            log_out!(@err res);
            res.success()?.out.parse()
        }

        /*
        /// Extract symbols
        async fn nm(self, file: String, opts: qjs::Opt<NmOptions>) -> Result<Vec<String>> {
            let args = &["--print-file-name", "--print-size", "--line-numbers", ""];
            let out = exec_out(&self.0.cfg.nm, args).await?.success()?.out;
        }

        /// Dump objects
        async fn objdump(self, file: String) -> Result<String>;
        */
    }

    #[quickjs(rename = "LdScript")]
    pub async fn ld_script(
        outdir: Directory,
        name: String,
        opts: LdScript,
        incs: qjs::Opt<Set<Artifact<Input, Actual>>>,
    ) -> Result<Artifact<Input, Actual>> {
        LdScriptInternal::create(outdir, name, opts, incs.0.unwrap_or_default()).await
    }
}
