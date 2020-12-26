use crate::{Artifact, ArtifactStore, Output, Result, Set, Time};
use futures::future;

impl ArtifactStore {
    pub async fn process(self, goals: Set<String>) -> Result<()> {
        self.remove_expired();

        self.init_artifacts().await?;

        process_artifacts(
            self.phony
                .read()
                .iter()
                .filter(|artifact| goals.contains(artifact.name())),
        )
        .await;

        Ok(())
    }

    fn remove_expired(&self) {
        self.actual.write().remove_expired();
        self.phony.write().remove_expired();
    }

    async fn init_artifacts(&self) -> Result<()> {
        future::join_all(self.actual.read().iter().map(|artifact| artifact.init()))
            .await
            .into_iter()
            .collect::<Result<_>>()?;
        Ok(())
    }
}

async fn process_artifacts<K, I: Iterator<Item = Artifact<(), K>>>(artifacts: I) {
    for artifact in artifacts {}
}
