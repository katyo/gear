mod cmdline;

use async_std::{fs::File, io::ReadExt};
use gear::{qjs, Result};
use std::{collections::HashMap, env};

#[paw::main]
#[async_std::main]
async fn main(args: cmdline::Args) -> Result<()> {
    log::debug!("Set log level to `{}`", args.log_level);
    env::set_var("LOG_LEVEL", &args.log_level);
    pretty_env_logger::init_custom_env("LOG_LEVEL");

    log::debug!("Set current dir to `{}`", args.source_dir.to_str().unwrap());
    env::set_current_dir(&args.source_dir);

    let vars = args
        .input
        .iter()
        .map(|item| item.to_pair())
        .filter(|item| item.is_some())
        .map(|item| item.unwrap())
        .collect::<HashMap<_, _>>();
    log::debug!("Captured vars `{:?}`", vars);

    let goals = args
        .input
        .iter()
        .map(|item| item.to_name())
        .filter(|item| item.is_some())
        .map(|item| item.unwrap())
        .collect::<Vec<_>>();
    log::debug!("Captured goals `{:?}`", goals);

    let rt = qjs::Runtime::new()?;
    let ctx = qjs::Context::full(&rt)?;

    rt.set_loader(
        (
            qjs::BuiltinResolver::default()
                .with_module("gear")
                .with_module("toolchain")
                .with_module("system"),
            qjs::FileResolver::default(),
        ),
        (
            qjs::ModuleLoader::default()
                .with_module(
                    "gear",
                    (
                        gear::DirectoryJs,
                        gear::ArtifactJs,
                        gear::ScopeJs,
                        gear::BuilderJs,
                    ),
                )
                .with_module("toolchain", gear::GccJs)
                .with_module("system", gear::SystemJs),
            qjs::ScriptLoader::default(),
        ),
    );

    rt.spawn_pending_jobs(None);

    ctx.with(|ctx| ctx.globals().init_def::<gear::ConsoleJs>())?;

    let artifacts = gear::ArtifactStore::default();
    let current_dir = gear::Directory::new(&artifacts, "");
    let root_scope = gear::Scope::new(&artifacts, "");

    ctx.with(|ctx| -> qjs::Result<()> {
        let globals = ctx.globals();
        globals.prop("root", qjs::Accessor::from(move || root_scope.clone()))?;
        globals.prop("base", qjs::Accessor::from(move || current_dir.clone()))?;
        Ok(())
    })?;

    let rules_name = args.rules_file.to_str().unwrap();

    log::debug!("Read rules file `{}`", rules_name);

    let mut file = File::open(&args.rules_file).await?;
    let mut src = String::new();
    file.read_to_string(&mut src).await?;

    let pend = ctx
        .with(move |ctx| -> qjs::Result<qjs::Promise<()>> {
            log::debug!("Compile rules file `{}`", rules_name);

            let module = qjs::Module::new(ctx, rules_name, src)?.eval()?;

            let default: qjs::Value = module.get("default")?;

            if default.is_function() {
                default.as_function().unwrap().call(())?
            } else {
                default
            }
            .get()
        })
        .map_err(|error| {
            log::error!(
                "Error when evaluating rules file `{}`: {}",
                rules_name,
                error
            );
            error
        })?;

    if let Err(error) = pend.await {
        log::error!(
            "Error when evaluating rules file `{}`: {}",
            rules_name,
            error
        );
    } else {
        log::debug!("Success");
    }

    Ok(())
}
