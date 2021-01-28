mod cmdline;

#[cfg(feature = "watch")]
mod watcher;

#[cfg(feature = "webui")]
mod server;

use async_std::{
    channel::{unbounded, Sender},
    fs::File,
    io::ReadExt,
};
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

    let values = match args.find_config().await {
        Some(path) => {
            log::debug!("Load config file `{}`", path);
            let mut values = gear::ValueStore::new(path)?;
            values.load().await?;
            values
        }
        None => {
            log::warn!("Unable to locate config file. Use defaults.");
            gear::ValueStore::new(args.default_config())?
        }
    };
    let config = values.path().display().to_string();

    let props = Props {
        file,
        config,
        paths,
        goals,
        base,
        dest,
    };

    Main::run(props, values, args).await?;

    Ok(())
}

struct Main;

impl Main {
    async fn run(props: Props, values: gear::ValueStore, args: Args) -> Result<()> {
        let props = Ref::new(props);
        let variables = gear::VariableStore::new(values, args.get_vars());
        let artifacts = gear::ArtifactStore::default();
        let store = gear::Store::new(variables, artifacts);
        let scope = gear::Scope::new_root(store);
        let (sender, receiver) = unbounded();

        #[cfg(feature = "webui")]
        if let Some(url) = &args.webui {
            server::Server::new(receiver, scope.clone()).spawn(url);
        }

        loop {
            let state = State::new(props.clone(), scope.clone(), sender.clone())?;

            state.load_rules().await?;

            if args.completions.is_some() {
                args.gen_completions();
            } else if let Some(print) = args.get_print() {
                state.print_db(print).await?;
            } else {
                if let Err(error) = state.sender.send(Event::RulesUpdate).await {
                    log::error!("Unable to send rules update event due to: {}", error);
                }

                let jobs = args.get_jobs();

                #[cfg(not(feature = "watch"))]
                state.build_rules(jobs, args.dry_run).await?;

                #[cfg(feature = "watch")]
                if args.watch {
                    // do not panic when rules fails to build completely
                    if let Err(error) = state.build_rules(jobs, args.dry_run).await {
                        eprintln!("{}", error);
                    }

                    if state.watch_inputs(jobs, args.dry_run).await? {
                        log::debug!("Reloading rules");
                        continue;
                    }
                } else {
                    state.build_rules(jobs, args.dry_run).await?;
                }
            }

            break;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub enum Event {
    RulesUpdate,
    RuleStateChange(gear::RuleStateChange),
}

struct Props {
    file: String,
    config: String,
    paths: Vec<String>,
    goals: Set<String>,
    base: String,
    dest: String,
}

#[derive(qjs::IntoJs)]
struct Environ {
    pub root: gear::Scope,
    pub base: gear::Directory,
    pub dest: gear::Directory,
}

impl Environ {
    fn new(state: &State) -> Self {
        let root = state.scope.clone();
        let base = gear::Directory::new(&root, &state.props.base);
        let dest = gear::Directory::new(&root, &state.props.dest);
        Self { root, base, dest }
    }
}

struct State {
    props: Ref<Props>,
    sender: Sender<Event>,
    rt: qjs::Runtime,
    ctx: qjs::Context,
    compile: qjs::Compile,
    scope: gear::Scope,
}

impl State {
    pub fn new(props: Ref<Props>, scope: gear::Scope, sender: Sender<Event>) -> Result<Self> {
        let (rt, ctx, compile) = Self::init_js(&props.paths)?;

        Ok(Self {
            props,
            sender,
            rt,
            ctx,
            compile,
            scope,
        })
    }

    fn init_js<P: AsRef<str>>(paths: &[P]) -> Result<(qjs::Runtime, qjs::Context, qjs::Compile)> {
        let rt = qjs::Runtime::new()?;
        let ctx = qjs::Context::full(&rt)?;
        let compile = qjs::Compile::new();

        rt.set_max_stack_size(4 << 20);
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
                            gear::VariableJs,
                            gear::DirectoryJs,
                            gear::ArtifactJs,
                            gear::ScopeJs,
                            gear::RuleJs,
                        ),
                    )
                    .with_module("toolchain", gear::CompilerJs)
                    .with_module("system", gear::SystemJs),
                qjs::ScriptLoader::default(),
            ),
        );

        rt.spawn_executor::<qjs::AsyncStd>();

        ctx.with(|ctx| -> Result<()> {
            let globals = ctx.globals();
            globals.init_def::<gear::ExtensionsJs>()?;
            globals.init_def::<gear::ConsoleJs>()?;
            Ok(())
        })?;

        Ok((rt, ctx, compile))
    }

    pub async fn load_rules(&self) -> Result<()> {
        self.scope.reset();

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
                default.as_function().unwrap().call((Environ::new(self),))?
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

    pub async fn build_rules(&self, jobs: usize, dry_run: bool) -> Result<()> {
        log::debug!("Build goals: {:?}", self.props.goals);
        let store: &gear::ArtifactStore = self.scope.as_ref();
        let sender = self.sender.clone();
        store
            .process(&self.props.goals, jobs, dry_run, move |event| {
                let sender = sender.clone();
                async move {
                    if let Err(error) = sender.send(Event::RuleStateChange(event)).await {
                        log::error!("Unable to send rule state change event due to: {}", error);
                    }
                }
            })
            .await
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
                    .chain(Some(self.props.file.as_str()))
                    .chain(Some(self.props.config.as_str()))
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
