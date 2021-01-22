mod deps_parser;

use crate::{
    system::{read_file, Path},
    Actual, Artifact, ArtifactStore, Input, Result, Set,
};
use futures::future::join_all;
use std::str::from_utf8;

use deps_parser::parse_deps;

impl ArtifactStore {
    pub async fn read_deps(
        &self,
        path: impl AsRef<Path>,
        filter: impl Fn(&String) -> bool,
    ) -> Result<Set<Artifact<Input, Actual>>> {
        let data = read_file(path).await?;
        let data = from_utf8(&data)?;
        let (_obj, deps) = parse_deps(&data)?;
        let deps = join_all(deps.into_iter().filter(filter).map(|path| {
            let store = self.clone();
            async move {
                let artifact = Artifact::new(store, path, "")?;
                artifact.init().await?;
                Ok(artifact)
            }
        }))
        .await
        .into_iter()
        .collect::<Result<_>>()?;

        Ok(deps)
    }
}
