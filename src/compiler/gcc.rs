use super::Config;
use crate::system::{access, exec_out, which, AccessMode, PathBuf};
use crate::{qjs, Ref, Result};

pub(self) struct Internal {
    path: PathBuf,
    version: String,
    machine: String,
}

impl Internal {
    pub async fn from_path<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let path = path.into();

        let version = exec_out(&path, &["-dumpversion"]).await?.0.trim().into();
        let machine = exec_out(&path, &["-dumpmachine"]).await?.0.trim().into();

        Ok(Self {
            path,
            version,
            machine,
        })
    }

    pub async fn new(config: Config) -> Result<Self> {
        let path = if let Some(path) = &config.path {
            PathBuf::from(path)
        } else {
            let name = if let Some(triple) = &config.triple {
                format!("{}-gcc", triple)
            } else if let Some(name) = &config.name {
                name.clone()
            } else {
                "gcc".into()
            };
            which(&name)
                .await
                .ok_or_else(|| format!("Unable to find executable `{}`", name))?
        };
        if !access(&path, AccessMode::EXECUTE).await {
            return Err(format!("Unable to get access to executable `{}` ", path.display()).into());
        }
        Self::from_path(path).await
    }
}

#[derive(Clone)]
pub struct Gcc(Ref<Internal>);

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Gcc {
        pub async fn new() -> Self {
            unimplemented!();
        }

        pub async fn config(config: qjs::Opt<Config>) -> Result<Self> {
            let config = config.0.unwrap_or_default();
            let intern = Internal::new(config).await?;
            Ok(Self(Ref::new(intern)))
        }

        /*pub async fn config(&self, config: qjs::Opt<Config>) -> Result<Self> {
            let config = config.0.unwrap_or_default();
            let intern = Internal::new(config).await?;
            Ok(Self(Ref::new(intern)))
        }*/

        #[quickjs(get, enumerable)]
        pub fn path(&self) -> String {
            self.0.path.display().to_string()
        }

        #[quickjs(get, enumerable)]
        pub fn version(&self) -> String {
            self.0.version.clone()
        }

        #[quickjs(get, enumerable)]
        pub fn machine(&self) -> String {
            self.0.machine.clone()
        }

        #[quickjs(get, enumerable)]
        pub async fn sysroot(&self) -> Result<String> {
            Ok(exec_out(&self.0.path, &["-print-sysroot"])
                .await?
                .0
                .trim()
                .into())
        }

        #[quickjs(get, enumerable)]
        pub async fn search_dirs(&self) -> Result<Vec<String>> {
            Ok(exec_out(&self.0.path, &["-print-search-dirs"])
                .await?
                .0
                .trim()
                .split(':')
                .map(|path| path.into())
                .collect())
        }
    }
}
