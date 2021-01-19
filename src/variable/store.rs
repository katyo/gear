mod json;

#[cfg(feature = "yaml")]
mod yaml;

#[cfg(feature = "toml")]
mod toml;

use crate::{system::PathBuf, Result, Value};
use async_std::fs::{read, write};

pub(self) trait ValueStoreApi {
    fn load(&mut self, data: &[u8]) -> Result<()>;
    fn save(&self) -> Result<Vec<u8>>;

    fn get(&self, path: &[&str]) -> Option<Value>;
    fn set(&mut self, path: &[&str], value: Option<&Value>);
}

pub struct ValueStore {
    /// Config path
    path: PathBuf,
    /// Config API
    api: Box<dyn ValueStoreApi + Send + Sync>,
}

impl ValueStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let extension = path
            .extension()
            .ok_or_else(|| format!("Config file `{}` should has extension", path.display()))?
            .to_str()
            .ok_or_else(|| "Invalid config file extension")?;

        let api: Box<dyn ValueStoreApi + Send + Sync> = match extension {
            "json" => Box::new(self::json::ValueStore::default()),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Box::new(self::yaml::ValueStore::default()),
            #[cfg(feature = "toml")]
            "toml" => Box::new(self::toml::ValueStore::default()),
            _ => return Err(format!("Unsupported config file extension `{}`", extension).into()),
        };

        Ok(Self { path, api })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let path = name.split('.').collect::<Vec<_>>();
        self.api.get(&path)
    }

    pub fn set(&mut self, name: &str, val: Option<&Value>) {
        let path = name.split('.').collect::<Vec<_>>();
        self.api.set(&path, val);
    }

    pub async fn load(&mut self) -> Result<()> {
        let data = read(&self.path).await?;
        self.api.load(&data)
    }

    pub async fn save(&self) -> Result<()> {
        let data = self.api.save()?;
        write(&self.path, data).await?;
        Ok(())
    }
}
