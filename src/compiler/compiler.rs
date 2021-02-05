use super::{
    CInputKind, COutputKind, CompilerKind, DCompilerKind, DepKind, DetectOpts, FileKind,
    FormatArgs, LdScript, PlatformKind, SizeInfo, ToolchainOpts,
};
use crate::{
    qjs,
    system::{check_access, exec_out, which_any, write_file, AccessMode, Path, PathBuf},
    Actual, Artifact, ArtifactStore, BoxedFuture, DataHasher, Diagnostics, Directory, Input, Mut,
    Output, Ref, Result, Rule, RuleApi, Set, WeakArtifact,
};
use futures::future::{join_all, FutureExt};
use std::iter::once;

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

#[derive(Hash)]
struct PropsInternal {
    /// C compiler path
    cc: String,
    /// D compiler path
    dc: Option<String>,
    /// Archiver path
    ar: String,
    /// Binutil nm path
    nm: String,
    /// Binutil size path
    size: String,
    /// Binutil strip path
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
        let kind: CompilerKind = path.parse()?;

        let (version, target, tools, dc) = match kind {
            CompilerKind::Gcc => {
                let mut version = exec_out(path, &["-dumpversion"]).await?.success()?.out;
                version.retain(|c| c != '\n');

                log::debug!("gcc version: {}", version);

                let mut target = exec_out(path, &["-dumpmachine"]).await?.success()?.out;
                target.retain(|c| c != '\n');

                log::debug!("gcc target: {}", target);

                async fn find_tool(path: &str, target: &str, name: &str) -> Result<String> {
                    let pre_path = &path[..path.len() - 3];
                    let path0 = format!("{}{}", pre_path, name);
                    let path1 = format!("{}-{}", target, name);
                    let paths = [path0.as_str(), path1.as_str(), name];

                    let path = which_any(if pre_path.ends_with("-") {
                        &paths[..2]
                    } else {
                        &paths[..]
                    })
                    .await
                    .ok_or_else(|| format!("Unable to find `{}`", name))?;
                    check_access(&path, AccessMode::EXECUTE).await?;
                    Ok(path.display().to_string())
                }

                let tools = join_all(
                    [
                        "gcc-ar", "gcc-nm", "size", "strip", "objcopy", "objdump", "readelf",
                    ]
                    .iter()
                    .map(|name| find_tool(&path, &target, name)),
                )
                .await;

                let gdc = find_tool(&path, &target, "gdc")
                    .await
                    .map_err(|error| {
                        log::warn!("{}", error);
                    })
                    .ok();

                (version, target, tools, gdc)
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
                    check_access(&path, AccessMode::EXECUTE).await?;
                    Ok(path)
                }

                let tools = join_all(
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
                .await;

                let ldc = find_tool(&path, "ldc2")
                    .await
                    .map_err(|error| {
                        log::warn!("{}", error);
                    })
                    .ok();

                (version, target, tools, ldc)
            }
        };

        let mut paths = tools.into_iter().collect::<Result<Vec<_>>>()?.into_iter();

        let ar = paths.next().unwrap();
        let nm = paths.next().unwrap();
        let size = paths.next().unwrap();
        let strip = paths.next().unwrap();
        let objcopy = paths.next().unwrap();
        let objdump = paths.next().unwrap();
        let readelf = paths.next().unwrap();

        let platform = PlatformKind::from_target(&target)?;

        Ok(Self {
            cc: opts.compiler.clone(),
            dc,
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

impl CompilerConfig {
    pub fn hash(&self) -> String {
        DataHasher::hash_base64_string(&(&self.0.props, &self.0.opts))
    }

    pub fn base_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        self.0.opts.base.fmt_args(&mut out);
        out
    }

    pub fn c_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        (&self.0.opts.base, &self.0.opts.cc, &self.0.opts.c).fmt_args(&mut out);
        out
    }

    pub fn cxx_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        (&self.0.opts.base, &self.0.opts.cc, &self.0.opts.cxx).fmt_args(&mut out);
        out
    }

    pub fn d_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        (self.0.props.kind, &self.0.opts.base, &self.0.opts.d).fmt_args(&mut out);
        out
    }

    pub fn link_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        (&self.0.opts.base, &self.0.opts.link).fmt_args(&mut out);
        out
    }

    pub fn dump_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        self.0.opts.dump.fmt_args(&mut out);
        out
    }

    pub fn strip_opts(&self) -> Vec<String> {
        let mut out = Vec::default();
        self.0.opts.strip.fmt_args(&mut out);
        out
    }
}

#[derive(Clone)]
pub(self) struct Internal {
    props: Ref<PropsInternal>,
    opts: ToolchainOpts,
}

impl Internal {
    pub async fn detect(opts: DetectOpts) -> Result<Self> {
        let props = PropsInternal::new(opts).await?;

        /*let diag_opts: &[&str] = match props.kind {
            CompilerKind::Gcc => &[
                //"-fno-diagnostics-show-caret",
                //"-fno-diagnostics-color",
                //"-fdiagnostics-show-option",
                "-fdiagnostics-parseable-fixits",
            ],
            CompilerKind::Llvm => &[
                //"-fdiagnostics-format=clang",
                //"-fdiagnostics-print-source-range-info",
                //"-fno-caret-diagnostics",
                //"-fno-color-diagnostics",
                //"-fdiagnostics-show-option",
                "-fdiagnostics-parseable-fixits",
            ],
        };

        for opts in &[&mut compile_opts, &mut link_opts] {
            opts.extend(diag_opts);
        }*/

        Ok(Self {
            props: Ref::new(props),
            opts: Default::default(),
        })
    }

    pub fn config(&self, new_opts: ToolchainOpts) -> Self {
        let mut opts = self.opts.clone();
        opts.extend(Some(new_opts));

        Self {
            props: self.props.clone(),
            opts,
        }
    }
}

#[derive(Debug, Clone, qjs::FromJs, Default)]
pub struct CompileOptions {
    pub input: Option<CInputKind>,
    pub output: COutputKind,
}

pub(self) struct CompileInternal {
    cfg: CompilerConfig,
    store: ArtifactStore,
    in_kind: CInputKind,
    out_kind: COutputKind,
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

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        async move {
            log::debug!("Compile::invoke");
            Ok(if let Some(dst) = self.dst.try_ref() {
                let deps_name = self.dep.display().to_string();
                let src = &self.src;
                let mut dep_kind = DepKind::default();

                let (cmd, args) = if self.in_kind == CInputKind::D {
                    let mut args = self.cfg.d_opts();

                    match DCompilerKind::from(self.cfg.0.props.kind) {
                        DCompilerKind::Gdc => {
                            args.push(
                                match self.out_kind {
                                    COutputKind::Asm => "-S",
                                    COutputKind::Obj => "-c",
                                    _ => unreachable!(),
                                }
                                .into(),
                            );
                            args.push("-MMD".into());
                            args.push("-MF".into());
                            args.push(deps_name);
                            args.push("-o".into());
                            args.push(dst.name().clone());
                            args.push(src.name().clone());
                        }
                        DCompilerKind::Ldc => {
                            args.push("--verror-style=gnu".into());
                            args.push(format!("--mtriple={}", self.cfg.0.props.target));
                            args.push(format!(
                                "--output-{}",
                                match self.out_kind {
                                    COutputKind::Asm => "s",
                                    COutputKind::Obj => "o",
                                    COutputKind::Ir => "ll",
                                    COutputKind::Bc => "bc",
                                    _ => unreachable!(),
                                }
                            ));
                            args.push(format!("--deps={}", deps_name));
                            dep_kind = DepKind::D;
                            args.push("--op".into());
                            args.push(format!("--of={}", dst.name()));
                            args.push(src.name().clone());
                        }
                    }

                    (self.cfg.0.props.dc.as_ref().unwrap(), args)
                } else {
                    fn with_lang(lang: &str, mut args: Vec<String>) -> Vec<String> {
                        args.push(format!("-x{}", lang));
                        args
                    }

                    let mut args = match self.in_kind {
                        CInputKind::C => with_lang("c", self.cfg.c_opts()),
                        CInputKind::Asm => with_lang("assembler-with-cpp", self.cfg.c_opts()),
                        CInputKind::Cxx => with_lang("c++", self.cfg.cxx_opts()),
                        _ => unreachable!(),
                    };

                    if matches!(self.cfg.0.props.kind, CompilerKind::Llvm) {
                        args.push(format!("--target={}", self.cfg.0.props.target));

                        if matches!(self.out_kind, COutputKind::Ir | COutputKind::Bc) {
                            args.push("--emit-llvm".into());
                        }
                    }

                    args.push(
                        match self.out_kind {
                            COutputKind::Cpp => "-E",
                            COutputKind::Asm | COutputKind::Ir => "-S",
                            COutputKind::Obj | COutputKind::Bc => "-c",
                        }
                        .into(),
                    );

                    args.push("-MMD".into());
                    args.push("-MF".into());
                    args.push(deps_name);
                    args.push("-o".into());
                    args.push(dst.name().clone());
                    args.push(src.name().clone());

                    (&self.cfg.0.props.cc, args)
                };

                let res = exec_out(cmd, &args).await?;
                log_out!(res);

                let dep_path = &self.dep;
                if dep_path.is_file().await {
                    let src_name = self.src.name();
                    // reload generated deps
                    let incs = self
                        .store
                        .read_deps(dep_path, dep_kind, |src| src != src_name)
                        .await?;
                    *self.incs.write() = incs;
                }

                res.err.parse()?
            } else {
                Default::default()
            })
        }
        .boxed_local()
    }
}

#[derive(Debug, Clone, qjs::IntoJs)]
pub struct LinkOutput {
    pub out: Artifact<Input, Actual>,
    pub map: Artifact<Input, Actual>,
}

#[derive(Debug, Clone, Default)]
pub struct LinkOptions {
    output: FileKind,
    script: Option<Artifact<Input, Actual>>,
}

impl<'js> qjs::FromJs<'js> for LinkOptions {
    fn from_js(_ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let obj: qjs::Object = val.get()?;
        let output = if obj.contains_key("type")? {
            val.get()?
        } else {
            Default::default()
        };
        let script = obj.get("script")?;

        Ok(Self { output, script })
    }
}

pub(self) struct LinkInternal {
    cfg: CompilerConfig,
    out_kind: FileKind,
    objs: Set<Artifact<Input, Actual>>,
    script: Option<Artifact<Input, Actual>>,
    out: WeakArtifact<Output, Actual>,
    map: WeakArtifact<Output, Actual>,
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

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        async move {
            log::debug!("Link::invoke");
            Ok(if let Some(out) = self.out.try_ref() {
                let (cmd, mut args) = if matches!(self.out_kind, FileKind::Static { .. }) {
                    (&self.cfg.0.props.ar, vec!["cr".into(), out.name().clone()])
                } else {
                    let mut args = self.cfg.link_opts();

                    args.push("-o".into());
                    args.push(out.name().clone());

                    if matches!(self.out_kind, FileKind::Dynamic { .. }) {
                        args.push("-shared".into());
                    }

                    if let Some(script) = &self.script {
                        args.push("-T".into());
                        args.push(script.name().clone());
                    }

                    if let Some(map) = self.map.try_ref() {
                        args.push(format!("-Wl,-Map,{}", map.name()));
                    }

                    (&self.cfg.0.props.cc, args)
                };

                args.extend(self.objs.iter().map(|obj| obj.name().clone()));

                let res = exec_out(cmd, &args).await?;
                log_out!(res);
                res.err.parse()?
            } else {
                Default::default()
            })
        }
        .boxed_local()
    }
}

#[derive(Debug, Clone, qjs::IntoJs)]
pub struct StripOutput {
    pub out: Artifact<Input, Actual>,
    pub strip: Option<Artifact<Input, Actual>>,
}

pub(self) struct StripInternal {
    cfg: CompilerConfig,
    obj: Artifact<Input, Actual>,
    out: WeakArtifact<Output, Actual>,
    strip_out: Option<WeakArtifact<Output, Actual>>,
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
                self.strip_out
                    .as_ref()
                    .and_then(|out| out.try_ref().map(|input| input.into_kind_any())),
            )
            .collect()
    }

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        async move {
            log::debug!("Strip::invoke");
            if let Some(out) = self.out.try_ref() {
                let mut args = self.cfg.strip_opts();

                if let Some(strip_out) = self.strip_out.as_ref() {
                    if let Some(strip_out) = strip_out.try_ref() {
                        args.push("-o".into());
                        args.push(strip_out.name().clone());
                    }
                }

                args.push(out.name().clone());

                let res = exec_out(&self.cfg.0.props.strip, &args).await?;
                log_out!(res);
                res.success()?;
            }
            Ok(Default::default())
        }
        .boxed_local()
    }
}

impl CompilerConfig {
    async fn compile(
        self,
        src: Artifact<Input, Actual>,
        out_dir: Directory,
        opts: Option<CompileOptions>,
    ) -> Result<Artifact<Input, Actual>> {
        let opts = opts.unwrap_or_default();

        let out_kind = opts.output;
        let src_name = src.name().clone();

        let in_kind = if let Some(kind) = opts.input {
            kind
        } else {
            CInputKind::from_name(&src_name)?
        };

        if matches!(in_kind, CInputKind::D) {
            if self.0.props.dc.is_none() {
                Err(format!("No D compiler found to build `{}`", src_name))?;
            }

            if matches!(out_kind, COutputKind::Cpp) {
                Err(format!(
                    "Unable preprocess `{}` because D-lang does not support C-preprocessor",
                    src_name
                ))?;
            }
        }

        if matches!(self.0.props.kind, CompilerKind::Gcc)
            && matches!(out_kind, COutputKind::Ir | COutputKind::Bc)
        {
            Err(format!(
                "Output intermediate reprepresentation for `{}` from GCC is experimental and does not supported yet",
                src_name
            ))?;
        }

        let hash = self.hash();
        let out_dir = out_dir.child(&hash);

        let dst_ext = out_kind.make_extension(&src_name);
        let dst_name = format!("{}.{}", src_name, dst_ext);
        let dst = out_dir.output(dst_name).await?;

        let dep_name = format!("{}.dep", dst.name());
        let dep_path = PathBuf::from(&dep_name);

        let dep_kind = if matches!(in_kind, CInputKind::D)
            && matches!(self.0.props.kind, CompilerKind::Llvm)
        {
            DepKind::D
        } else {
            DepKind::default()
        };

        let store: &ArtifactStore = out_dir.as_ref();
        let incs = if dep_path.is_file().await {
            // preload already generated deps
            store
                .read_deps(&dep_path, dep_kind, |src| src != &src_name)
                .await?
        } else {
            // deps will be generated under compilation
            Default::default()
        };

        log::debug!("Compile::new");

        let rule = Ref::new(CompileInternal {
            cfg: self.clone(),
            store: store.clone(),
            in_kind,
            out_kind,
            src,
            dep: dep_path,
            incs: Mut::new(incs),
            dst: dst.weak(),
        });

        dst.set_rule(Rule::from_api(rule));

        Ok(dst.into())
    }

    async fn link(
        self,
        objs: Set<Artifact<Input, Actual>>,
        out_dir: Directory,
        out_name: impl AsRef<str>,
        opts: Option<LinkOptions>,
    ) -> Result<LinkOutput> {
        let opts = opts.unwrap_or_default();

        let script = opts.script;
        let out_kind = opts.output;

        let out_name = out_kind.file_name(&self.0.props.platform, out_name);
        let out = out_dir.output(&out_name).await?;

        let map_name = format!("{}.map", out_name);
        let map = out_dir.output(map_name).await?;

        log::debug!("Link::new");

        let rule = Ref::new(LinkInternal {
            cfg: self.clone(),
            out_kind,
            objs,
            script,
            out: out.weak(),
            map: map.weak(),
        });

        let rule = Rule::from_api(rule);

        out.set_rule(rule.clone());
        map.set_rule(rule);

        Ok(LinkOutput {
            out: out.into(),
            map: map.into(),
        })
    }

    async fn strip(
        self,
        obj: Artifact<Input, Actual>,
        out_dir: Directory,
        strip_dir: Option<Directory>,
    ) -> Result<StripOutput> {
        let obj_name = obj.name().clone();

        let out_name = Path::new(&obj_name)
            .file_name()
            .ok_or_else(|| format!("Unable to determine file name to strip `{}`", obj_name))?
            .to_str()
            .unwrap();
        let out = out_dir.output(out_name).await?;
        let strip_out = if let Some(strip_dir) = &strip_dir {
            Some(strip_dir.output(out_name).await?)
        } else {
            None
        };

        log::debug!("Strip::new");

        let rule = Ref::new(StripInternal {
            cfg: self.clone(),
            obj,
            out: out.weak(),
            strip_out: strip_out.as_ref().map(|out| out.weak()),
        });

        let rule = Rule::from_api(rule);
        if let Some(strip_out) = &strip_out {
            out.set_rule(rule.clone());
            strip_out.set_rule(rule);
        } else {
            out.set_rule(rule);
        }

        Ok(StripOutput {
            out: out.into(),
            strip: strip_out.map(|out| out.into()),
        })
    }

    pub async fn get(&self, name: impl AsRef<str>) -> Result<String> {
        let mut args = self.base_opts();
        args.push(format!("-print-{}", name.as_ref()));

        Ok(exec_out(&self.0.props.cc, &args)
            .await?
            .success()?
            .out
            .trim()
            .into())
    }

    #[inline]
    pub async fn sysroot(&self) -> Result<String> {
        self.get("sysroot").await
    }

    #[inline]
    pub async fn multidir(&self) -> Result<String> {
        self.get("multi-directory").await
    }

    #[inline]
    pub async fn builtins(&self) -> Result<String> {
        self.get("libgcc-file-name").await
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

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        async move {
            log::debug!("LdScript::invoke");

            if let Some(out) = self.out.try_ref() {
                let mut data = LdScript::default();
                data.includes = self.incs.iter().map(|inc| inc.name().into()).collect();
                let content = format!("{}{}", self.data, data);
                write_file(out.name(), content).await?;
            }

            Ok(Default::default())
        }
        .boxed_local()
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

        #[quickjs(get, enumerable)]
        pub fn cc_path(&self) -> &String {
            &self.0.props.cc
        }

        #[quickjs(get, enumerable)]
        pub fn ar_path(&self) -> &String {
            &self.0.props.ar
        }

        #[quickjs(get, enumerable)]
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

        #[quickjs(rename = "options", get, enumerable)]
        pub fn options_js(&self) -> ToolchainOpts {
            self.0.opts.clone()
        }

        #[quickjs(rename = "sysroot", get, enumerable)]
        pub async fn sysroot_js(self) -> Result<String> {
            self.sysroot().await
        }

        #[quickjs(rename = "multidir", get, enumerable)]
        pub async fn multidir_js(self) -> Result<String> {
            self.multidir().await
        }

        #[quickjs(rename = "builtins", get, enumerable)]
        pub async fn builtins_js(self) -> Result<String> {
            self.builtins().await
        }

        #[quickjs(get, enumerable)]
        pub async fn search_dirs(self) -> Result<Vec<String>> {
            let mut args = self.base_opts();
            args.push("-print-search-dirs".into());

            Ok(exec_out(&self.0.props.cc, &args)
                .await?
                .success()?
                .out
                .trim()
                .split(':')
                .map(|path| path.into())
                .collect())
        }

        #[quickjs(get, enumerable, hide)]
        pub fn hash(&self) -> String {}

        #[quickjs(get, enumerable, hide)]
        pub fn c_opts(&self) -> Vec<String> {}

        #[quickjs(get, enumerable, hide)]
        pub fn cxx_opts(&self) -> Vec<String> {}

        #[quickjs(get, enumerable, hide)]
        pub fn d_opts(&self) -> Vec<String> {}

        #[quickjs(get, enumerable, hide)]
        pub fn link_opts(&self) -> Vec<String> {}

        #[quickjs(get, enumerable, hide)]
        pub fn dump_opts(&self) -> Vec<String> {}

        #[quickjs(get, enumerable, hide)]
        pub fn strip_opts(&self) -> Vec<String> {}

        /// Compile source
        #[quickjs(rename = "compile")]
        pub async fn compile_js(
            self,
            out_dir: Directory,
            src: Artifact<Input, Actual>,
            opts: qjs::Opt<CompileOptions>,
        ) -> Result<Artifact<Input, Actual>> {
            self.compile(src, out_dir, opts.0).await
        }

        /// Link objects
        #[quickjs(rename = "link")]
        pub async fn link_js(
            self,
            out_dir: Directory,
            out_name: String,
            objs: Set<Artifact<Input, Actual>>,
            opts: qjs::Opt<LinkOptions>,
        ) -> Result<LinkOutput> {
            self.link(objs, out_dir, out_name, opts.0).await
        }

        /// Strip objects
        #[quickjs(rename = "strip")]
        pub async fn strip_js(
            self,
            obj: Artifact<Input, Actual>,
            out_dir: Directory,
            strip_dir: qjs::Opt<Directory>,
        ) -> Result<StripOutput> {
            self.strip(obj, out_dir, strip_dir.0).await
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
