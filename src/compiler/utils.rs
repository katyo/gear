mod deps_parser;
mod ld_script;
mod size_parser;

use crate::{
    system::{read_file, Path},
    Actual, Artifact, ArtifactStore, Input, Result, Set,
};
use futures::future::join_all;
use std::str::from_utf8;

pub use deps_parser::DepsInfo;
pub use ld_script::LdScript;
pub use size_parser::SizeInfo;

impl ArtifactStore {
    pub async fn read_deps(
        &self,
        path: impl AsRef<Path>,
        filter: impl Fn(&String) -> bool,
    ) -> Result<Set<Artifact<Input, Actual>>> {
        let data = read_file(path).await?;
        let data = from_utf8(&data)?;
        let info: DepsInfo = data.parse()?;
        let deps = join_all(
            info.deps
                .into_iter()
                .filter(filter)
                .map(|path| Artifact::<Input, Actual>::new_init(self.clone(), path, "")),
        )
        .await
        .into_iter()
        .collect::<Result<_>>()?;

        Ok(deps)
    }
}
