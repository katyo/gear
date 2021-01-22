use super::{CommonOpts, CompileOpts, DetectOpts, DumpOpts, FormatArgs, LinkOpts, ToolchainOpts};
use crate::{
    qjs,
    system::{exec_out, Path, PathBuf},
    Actual, Artifact, ArtifactStore, DataHasher, Directory, Input, Mut, Output, Ref, Result, Rule,
    RuleApi, Set, WeakArtifact,
};
use std::{future::Future, iter::once, pin::Pin};

#[derive(Clone)]
pub struct GccConfig(Ref<Internal>);

#[derive(Clone)]
pub(self) struct Internal {
    path: PathBuf,
    version: String,
    machine: String,
    opts: ToolchainOpts,
    compile_opts: Vec<String>,
    compile_hash: String,
    link_opts: Vec<String>,
    dump_opts: Vec<String>,
}

impl Internal {
    pub async fn from_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        let version = exec_out(&path, &["-dumpversion"])
            .await?
            .success()?
            .out
            .trim()
            .into();
        let machine = exec_out(&path, &["-dumpmachine"])
            .await?
            .success()?
            .out
            .trim()
            .into();

        let compile_opts = Vec::<String>::default();
        let compile_hash = DataHasher::hash_base64_string(&compile_opts);
        let link_opts = Default::default();
        let dump_opts = Default::default();

        Ok(Self {
            path,
            version,
            machine,
            opts: Default::default(),
            compile_opts,
            compile_hash,
            link_opts,
            dump_opts,
        })
    }

    pub async fn detect(opts: DetectOpts) -> Result<Self> {
        let path = opts.detect("gcc").await?;
        Self::from_path(path).await
    }

    pub fn config(&self, new_opts: ToolchainOpts) -> Self {
        let mut opts = self.opts.clone();
        opts.extend(Some(new_opts));

        let mut compile_opts = Vec::default();
        opts.common.fmt_args(&mut compile_opts);
        opts.compile.fmt_args(&mut compile_opts);

        let compile_hash = DataHasher::hash_base64_string(&compile_opts);

        let mut link_opts = Vec::default();
        opts.common.fmt_args(&mut link_opts);
        opts.link.fmt_args(&mut link_opts);

        let mut dump_opts = Vec::default();
        opts.dump.fmt_args(&mut dump_opts);

        Self {
            path: self.path.clone(),
            version: self.version.clone(),
            machine: self.machine.clone(),
            opts,
            compile_opts,
            compile_hash,
            link_opts,
            dump_opts,
        }
    }
}

pub(self) struct CompileInternal {
    cfg: GccConfig,
    args: Vec<String>,
    src: Artifact<Input, Actual>,
    dep: PathBuf,
    incs: Mut<Set<Artifact<Input, Actual>>>,
    obj: WeakArtifact<Output, Actual>,
}

impl RuleApi for Ref<CompileInternal> {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        once(&self.src)
            .chain(self.incs.read().iter())
            .map(|input| input.clone().into_kind_any())
            .collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.obj
            .try_ref()
            .map(|input| input.into_kind_any())
            .into_iter()
            .collect()
    }

    fn invoke(&self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        let this = self.clone();
        Box::pin(async move {
            let res = exec_out(&this.cfg.0.path, &this.args).await?;
            log::error!("{}", res.err);
            log::warn!("{}", res.out);
            res.success()?;
            Ok(())
        })
    }
}

impl CompileInternal {
    async fn instance(
        cfg: GccConfig,
        outdir: Directory,
        src: Artifact<Input, Actual>,
    ) -> Result<Artifact<Output, Actual>> {
        let src_name = src.name().clone();
        let cfg_hash = cfg.0.compile_hash.clone();
        let dep_name = format!("{}.{}.d", src_name, cfg_hash);
        let out_name = format!("{}.{}.o", src_name, cfg_hash);
        let obj = outdir.output(&out_name).await?;
        let store: &ArtifactStore = outdir.as_ref();

        let dep_path = PathBuf::from(&dep_name);
        let incs = if dep_path.is_file().await {
            // preload already generated deps
            store.read_deps(&dep_path, |src| src != &src_name).await?
        } else {
            // deps will be generated under compilation
            Default::default()
        };

        let mut args = vec![
            "-MMD".to_string(),
            "-MF".to_string(),
            dep_name,
            "-o".to_string(),
            out_name,
            src_name,
        ];
        args.extend(cfg.0.compile_opts.iter().cloned());

        log::debug!("GccCompile::new");

        let rule = Ref::new(Self {
            cfg,
            args,
            src,
            dep: dep_path,
            incs: Mut::new(incs),
            obj: obj.weak(),
        });

        obj.set_rule(Rule::from_api(rule));

        Ok(obj)
    }
}

impl Drop for CompileInternal {
    fn drop(&mut self) {
        log::debug!("GccCompile::drop");
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(rename = "Gcc", cloneable)]
    impl GccConfig {
        pub fn new() -> Self {
            unimplemented!();
        }

        pub async fn detect(opts: qjs::Opt<DetectOpts>) -> Result<Self> {
            let opts = opts.0.unwrap_or_default();
            let intern = Internal::detect(opts).await?;
            Ok(Self(Ref::new(intern)))
        }

        pub fn config(&self, opts: qjs::Opt<ToolchainOpts>) -> Result<Self> {
            let opts = opts.0.unwrap_or_default();
            let intern = self.0.config(opts);
            Ok(Self(Ref::new(intern)))
        }

        #[quickjs(get, enumerable)]
        pub fn path(&self) -> String {
            self.0.path.display().to_string()
        }

        #[quickjs(get, enumerable)]
        pub fn version(&self) -> &String {
            &self.0.version
        }

        #[quickjs(get, enumerable)]
        pub fn machine(&self) -> &String {
            &self.0.machine
        }

        #[quickjs(get, enumerable)]
        pub async fn sysroot(self) -> Result<String> {
            Ok(exec_out(&self.0.path, &["-print-sysroot"])
                .await?
                .success()?
                .out
                .trim()
                .into())
        }

        #[quickjs(get, enumerable)]
        pub async fn search_dirs(self) -> Result<Vec<String>> {
            Ok(exec_out(&self.0.path, &["-print-search-dirs"])
                .await?
                .success()?
                .out
                .trim()
                .split(':')
                .map(|path| path.into())
                .collect())
        }
    }
}
