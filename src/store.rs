use crate::{ArtifactStore, VariableStore};

#[derive(Clone)]
pub struct Store {
    variables: VariableStore,
    artifacts: ArtifactStore,
}

impl AsRef<VariableStore> for Store {
    fn as_ref(&self) -> &VariableStore {
        &self.variables
    }
}

impl AsRef<ArtifactStore> for Store {
    fn as_ref(&self) -> &ArtifactStore {
        &self.artifacts
    }
}

impl Store {
    pub fn new(variables: VariableStore, artifacts: ArtifactStore) -> Self {
        Self {
            variables,
            artifacts,
        }
    }

    pub fn reset(&self) {
        self.variables.reset();
        self.artifacts.reset();
    }
}
