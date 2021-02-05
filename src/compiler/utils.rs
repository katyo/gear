mod d_deps_parser;
mod deps_parser;
mod diag_parser;
mod ld_script;
mod size_parser;

use crate::{
    system::{read_file, Path},
    Actual, Artifact, ArtifactStore, Input, Result, Set,
};
use futures::future::join_all;
use std::str::from_utf8;

pub use d_deps_parser::DDepsInfo;
pub use deps_parser::DepsInfo;
pub use ld_script::LdScript;
pub use size_parser::SizeInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepKind {
    Make,
    D,
}

impl Default for DepKind {
    fn default() -> Self {
        Self::Make
    }
}

impl ArtifactStore {
    pub async fn read_deps(
        &self,
        path: impl AsRef<Path>,
        kind: DepKind,
        filter: impl Fn(&String) -> bool,
    ) -> Result<Set<Artifact<Input, Actual>>> {
        let data = read_file(path).await?;
        let data = from_utf8(&data)?;
        let list = match kind {
            DepKind::Make => data.parse::<DepsInfo>()?.deps,
            DepKind::D => data.parse::<DDepsInfo>()?.dep_sources(),
        };
        let deps = join_all(
            list.into_iter()
                .filter(filter)
                .map(|path| Artifact::<Input, Actual>::new_init(self.clone(), path, "")),
        )
        .await
        .into_iter()
        .collect::<Result<_>>()?;

        Ok(deps)
    }
}
