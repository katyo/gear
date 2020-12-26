mod cmdline;

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

    log::debug!("Set current dir to `{}`", args.dir.display());
    env::set_current_dir(&args.dir);

    let paths = args.get_paths().collect::<Vec<_>>();
    let vars = args.get_vars().collect::<Map<_, _>>();
    let goals = args.get_goals().collect::<Set<_>>();

    let file = args.find_file().await.ok_or_else(|| {
        log::error!("Unable to locate rules file");
        "Unable to locate rules file"
    })?;

    let main = Main::new(paths, vars, goals)?;

    main.load_rules(&file).await?;

    if args.completions.is_some() {
        args.gen_completions();
    } else if let Some(print) = args.get_print() {
        main.print_db(print).await?;
    } else {
        main.build_rules(args.dry_run).await?;
        if args.watch {
            main.watch_inputs(args.dry_run).await?;
        }
    }

    Ok(())
}

pub struct Main {
    vars: Map<String, String>,
    goals: Set<String>,
    rt: qjs::Runtime,
    ctx: qjs::Context,
    scope: gear::Scope,
}

impl Main {
    pub fn new(paths: Vec<String>, vars: Map<String, String>, goals: Set<String>) -> Result<Self> {
        log::debug!("Modules paths `{:?}`", paths);
        log::debug!("Captured vars `{:?}`", vars);
        log::debug!("Captured goals `{:?}`", goals);

        let (rt, ctx) = Self::init_js(paths)?;

        let artifacts = gear::ArtifactStore::default();
        let scope = gear::Scope::new(&artifacts, "");

        ctx.with({
            let root = scope.clone();
            let base = gear::Directory::new(&artifacts, "");
            move |ctx| -> qjs::Result<_> {
                let globals = ctx.globals();
                globals.prop("root", qjs::Accessor::from(move || root.clone()))?;
                globals.prop("base", qjs::Accessor::from(move || base.clone()))?;
                Ok(())
            }
        })?;

        Ok(Self {
            vars,
            goals,
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

    pub async fn build_rules(&self, dry_run: bool) -> Result<()> {
        Ok(())
    }

    pub async fn watch_inputs(&self, dry_run: bool) -> Result<()> {
        Ok(())
    }
}
