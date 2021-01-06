mod cmdline;

#[cfg(feature = "watch")]
mod watcher;

use async_std::{fs::File, io::ReadExt};
use cmdline::{Args, Print};
use gear::{qjs, Map, Ref, Result, Set};
use std::env;

#[paw::main]
#[async_std::main]
async fn main(args: Args) -> Result<()> {
    let log = args.get_log();
    env::set_var("GEAR_LOG", &log);
    pretty_env_logger::init_custom_env("GEAR_LOG");
    log::debug!("Set log filter `{}`", log);

    log::trace!("{:?}", args);

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

    let props = Ref::new(Props {
        file,
        paths,
        vars,
        goals,
        base,
        dest,
    });

    loop {
        let main = Main::new(props.clone())?;

        main.load_rules().await?;

        if args.completions.is_some() {
            args.gen_completions();
        } else if let Some(print) = args.get_print() {
            main.print_db(print).await?;
        } else {
            let jobs = args.get_jobs();
            main.init_rules(jobs).await?;
            main.build_rules(jobs, args.dry_run).await?;

            #[cfg(feature = "watch")]
            if args.watch {
                if main.watch_inputs(jobs, args.dry_run).await? {
                    log::debug!("Reloading rules");
                    continue;
                }
            }
        }

        break;
    }

    Ok(())
}

pub struct Props {
    file: String,
    paths: Vec<String>,
    vars: Map<String, String>,
    goals: Set<String>,
    base: String,
    dest: String,
}

pub struct Main {
    props: Ref<Props>,
    rt: qjs::Runtime,
    ctx: qjs::Context,
    compile: qjs::Compile,
    scope: gear::Scope,
}

impl Main {
    pub fn new(props: Ref<Props>) -> Result<Self> {
        let (rt, ctx, compile) = Self::init_js(&props.paths)?;

        let artifacts = gear::ArtifactStore::default();
        let scope = gear::Scope::new(&artifacts, "");

        ctx.with({
            let root = scope.clone();
            let base = gear::Directory::new(&artifacts, &props.base);
            let dest = gear::Directory::new(&artifacts, &props.dest);
            move |ctx| -> qjs::Result<_> {
                let globals = ctx.globals();
                globals.prop("root", qjs::Accessor::from(move || root.clone()))?;
                globals.prop("base", qjs::Accessor::from(move || base.clone()))?;
                globals.prop("dest", qjs::Accessor::from(move || dest.clone()))?;
                Ok(())
            }
        })?;

        Ok(Self {
            props,
            rt,
            ctx,
            compile,
            scope,
        })
    }

    fn init_js<P: AsRef<str>>(paths: &[P]) -> Result<(qjs::Runtime, qjs::Context, qjs::Compile)> {
        let rt = qjs::Runtime::new()?;
        let ctx = qjs::Context::full(&rt)?;

        rt.spawn_executor::<qjs::AsyncStd>();

        let compile = qjs::Compile::new();

        rt.set_loader(
            (
                qjs::BuiltinResolver::default()
                    .with_module("gear")
                    .with_module("toolchain")
                    .with_module("system"),
                compile.resolver(qjs::FileResolver::default().with_paths(paths)),
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

        Ok((rt, ctx, compile))
    }

    pub async fn load_rules(&self) -> Result<()> {
        let name = self.props.file.as_str();
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

        if let Err(error) = pend.await {
            log::error!("Error when running rules file `{}`: {}", name, error);
        } else {
            log::debug!("Success");
        }

        self.rt.idle().await;
        Ok(())
    }

    fn match_goal(&self, name: &str) -> bool {
        if self.props.goals.is_empty() {
            true
        } else {
            self.props.goals.iter().any(|goal| name.starts_with(goal))
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

    pub async fn init_rules(&self, _jobs: usize) -> Result<()> {
        log::debug!("Init goals: {:?}", self.props.goals);
        let store: &gear::ArtifactStore = self.scope.as_ref();
        store.prepare().await
    }

    pub async fn build_rules(&self, jobs: usize, dry_run: bool) -> Result<()> {
        log::debug!("Build goals: {:?}", self.props.goals);
        let store: &gear::ArtifactStore = self.scope.as_ref();
        store.process(&self.props.goals, jobs, dry_run).await
    }

    #[cfg(feature = "watch")]
    pub async fn watch_inputs(&self, jobs: usize, dry_run: bool) -> Result<bool> {
        use futures::StreamExt;
        use gear::system::Path;

        let (mut watcher, mut events) = watcher::Watcher::new()?;

        let base = Path::new(if self.props.base.is_empty() {
            "."
        } else {
            &self.props.base
        })
        .canonicalize()
        .await?;

        log::debug!("Watch directory `{}` for updates", base.display());
        watcher.watch(&base, true)?;

        let modules = gear::Ref::new(
            futures::future::join_all(
                self.compile
                    .modules()
                    .into_iter()
                    .map(|(_name, path)| path)
                    .chain(std::iter::once(self.props.file.as_str()))
                    .map(|path| async move {
                        let path = path.to_string();
                        let time = gear::system::modified(&Path::new(&path)).await?;
                        Ok((path, time))
                    }),
            )
            .await
            .into_iter()
            .collect::<Result<Map<_, _>>>()?,
        );

        log::trace!("Watch rules files: {:?}", modules);

        loop {
            match events.next().await {
                Some(Ok(entries)) => {
                    let paths = entries
                        .iter()
                        .filter_map(|(path, time)| {
                            path.strip_prefix(&base)
                                .ok()
                                .and_then(|path| path.to_str())
                                .map(|name| (name, Some(*time)))
                        })
                        .collect::<Vec<_>>();

                    log::trace!("Touched paths: {:?}", paths);

                    for (path, _time) in paths.iter() {
                        if let Some(old_time) = modules.get(*path) {
                            if let Ok(new_time) = gear::system::modified(&Path::new(path)).await {
                                if new_time > *old_time {
                                    // Rules modified so need reload
                                    return Ok(true);
                                }
                            } else {
                                // Rules file removed so need reload
                                return Ok(true);
                            }
                        }
                    }

                    let store: &gear::ArtifactStore = self.scope.as_ref();
                    match store.update_sources(paths).await {
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

        Ok(false)
    }
}
