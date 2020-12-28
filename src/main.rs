mod cmdline;

#[cfg(feature = "watch")]
mod watcher;

use async_std::{fs::File, io::ReadExt};
use cmdline::{Args, Print};
use gear::{qjs, Map, Result, Set};
use std::env;

#[paw::main]
#[async_std::main]
async fn main(args: Args) -> Result<()> {
    let log = args.get_log();
    env::set_var("GEAR_LOG", &log);
    pretty_env_logger::init_custom_env("GEAR_LOG");
    log::debug!("Set log filter `{}`", log);

    log::trace!("{:?}", args);

    //log::debug!("Set current dir to `{}`", args.dir.display());
    //env::set_current_dir(&args.dir);

    let paths = args.get_paths().collect::<Vec<_>>();
    log::debug!("Modules paths `{:?}`", paths);

    let vars = args.get_vars().collect::<Map<_, _>>();
    log::debug!("Captured vars `{:?}`", vars);

    let goals = args.get_goals().collect::<Set<_>>();
    log::debug!("Captured goals `{:?}`", goals);

    let base = args.get_base();
    log::debug!("Base directory `{}`", base);

    let dest = args.get_dest();
    log::debug!("Dest directory `{}`", dest);

    let file = args.find_file().await.ok_or_else(|| {
        log::error!("Unable to locate rules file");
        "Unable to locate rules file"
    })?;

    let main = Main::new(paths, vars, goals, base, dest)?;

    main.load_rules(&file).await?;

    if args.completions.is_some() {
        args.gen_completions();
    } else if let Some(print) = args.get_print() {
        main.print_db(print).await?;
    } else {
        let jobs = args.get_jobs();
        main.build_rules(jobs, args.dry_run).await?;

        #[cfg(feature = "watch")]
        if args.watch {
            main.watch_inputs(jobs, args.dry_run).await?;
        }
    }

    Ok(())
}

pub struct Main {
    vars: Map<String, String>,
    goals: Set<String>,
    base: String,
    rt: qjs::Runtime,
    ctx: qjs::Context,
    scope: gear::Scope,
}

impl Main {
    pub fn new(
        paths: Vec<String>,
        vars: Map<String, String>,
        goals: Set<String>,
        base: String,
        dest: String,
    ) -> Result<Self> {
        let (rt, ctx) = Self::init_js(paths)?;

        let artifacts = gear::ArtifactStore::default();
        let scope = gear::Scope::new(&artifacts, "");

        ctx.with({
            let root = scope.clone();
            let base = gear::Directory::new(&artifacts, &base);
            let dest = gear::Directory::new(&artifacts, dest);
            move |ctx| -> qjs::Result<_> {
                let globals = ctx.globals();
                globals.prop("root", qjs::Accessor::from(move || root.clone()))?;
                globals.prop("base", qjs::Accessor::from(move || base.clone()))?;
                globals.prop("dest", qjs::Accessor::from(move || dest.clone()))?;
                Ok(())
            }
        })?;

        Ok(Self {
            vars,
            goals,
            base,
            rt,
            ctx,
            scope,
        })
    }

    fn init_js(paths: Vec<String>) -> Result<(qjs::Runtime, qjs::Context)> {
        let rt = qjs::Runtime::new()?;
        let ctx = qjs::Context::full(&rt)?;

        rt.set_loader(
            (
                qjs::BuiltinResolver::default()
                    .with_module("gear")
                    .with_module("toolchain")
                    .with_module("system"),
                qjs::FileResolver::default().with_paths(paths),
            ),
            (
                qjs::ModuleLoader::default()
                    .with_module(
                        "gear",
                        (
                            gear::DirectoryJs,
                            gear::ArtifactJs,
                            gear::ScopeJs,
                            gear::RuleJs,
                        ),
                    )
                    .with_module("toolchain", gear::GccJs)
                    .with_module("system", gear::SystemJs),
                qjs::ScriptLoader::default(),
            ),
        );

        ctx.with(|ctx| ctx.globals().init_def::<gear::ConsoleJs>())?;

        Ok((rt, ctx))
    }

    pub async fn load_rules(&self, name: &str) -> Result<()> {
        log::debug!("Read rules file `{}`", name);

        let mut file = File::open(name).await?;
        let mut src = String::new();
        file.read_to_string(&mut src).await?;

        let pend = self.ctx.with(move |ctx| -> qjs::Result<qjs::Promise<()>> {
            log::debug!("Compile rules file `{}`", name);
            let module = qjs::Module::new(ctx, name, src)?;
            log::debug!("Evaluate rules file `{}`", name);
            let module = module.eval()?;

            let default: qjs::Value = module.get("default")?;

            if default.is_function() {
                default.as_function().unwrap().call(())?
            } else {
                default
            }
            .get()
        })?;

        let handle = self.rt.spawn_pending_jobs(Some(10));

        if let Err(error) = pend.await {
            log::error!("Error when running rules file `{}`: {}", name, error);
        } else {
            log::debug!("Success");
        }

        handle.await;
        Ok(())
    }

    fn match_goal(&self, name: &str) -> bool {
        if self.goals.is_empty() {
            true
        } else {
            self.goals.iter().any(|goal| name.starts_with(goal))
        }
    }

    pub async fn print_db(&self, print: Print) -> Result<()> {
        match print {
            Print::Goals => print!(
                "{}",
                gear::NodeDisplay((&self.scope, &|name: &str| self.match_goal(name)))
            ),
            Print::Graph => print!(
                "{}",
                gear::NodeDisplay((
                    {
                        let store: &gear::ArtifactStore = self.scope.as_ref();
                        store
                    },
                    &|name: &str| self.match_goal(name)
                ))
            ),
        }
        Ok(())
    }

    pub async fn build_rules(&self, jobs: usize, dry_run: bool) -> Result<()> {
        log::debug!("Build goals: {:?}", self.goals);
        let _handle = self.rt.spawn_pending_jobs(None);
        let store: &gear::ArtifactStore = self.scope.as_ref();
        store
            .process(jobs, |name: &str| self.match_goal(name))
            .await
    }

    #[cfg(feature = "watch")]
    pub async fn watch_inputs(&self, jobs: usize, dry_run: bool) -> Result<()> {
        use futures::StreamExt;
        use gear::system::Path;

        let (mut watcher, mut events) = watcher::Watcher::new()?;

        let base = Path::new(if self.base.is_empty() {
            "."
        } else {
            &self.base
        })
        .canonicalize()
        .await?;

        log::debug!("Watch directory `{}` for updates", base.display());
        watcher.watch(&base, true)?;

        loop {
            match events.next().await {
                Some(Ok(entries)) => {
                    let store: &gear::ArtifactStore = self.scope.as_ref();
                    match store
                        .update_sources(entries.iter().filter_map(|(path, time)| {
                            path.strip_prefix(&base)
                                .ok()
                                .and_then(|path| path.to_str())
                                .map(|name| (name, Some(*time)))
                        }))
                        .await
                    {
                        Ok(true) => self.build_rules(jobs, dry_run).await?,
                        Err(error) => {
                            log::error!("Errot then updating sources: {}", error);
                        }
                        _ => (),
                    }
                }
                Some(Err(error)) => {
                    log::error!("Watch error: {}", error);
                    break;
                }
                _ => (),
            }
        }

        watcher.unwatch(&base)?;

        Ok(())
    }
}
