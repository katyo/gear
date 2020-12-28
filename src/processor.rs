use crate::{Artifact, ArtifactStore, Output, Result, Rule, Set, Time};
use futures::future;
use std::collections::VecDeque;

impl ArtifactStore {
    async fn process_artifacts<K, I: Iterator<Item = Artifact<(), K>>>(
        &self,
        jobs: usize,
        artifacts: I,
    ) -> Result<()> {
        let mut queue = VecDeque::new();
        let mut schedule = |rule: Rule| queue.push_back(rule);
        for artifact in artifacts {
            artifact.process(&mut schedule);
        }
        log::trace!("Prepare pending");
        let mut opt_pending = (0..jobs)
            .into_iter()
            .filter_map(|_| {
                log::trace!("Prepare pending rule");
                let mut out = 0;
                while !queue.is_empty() {
                    if let Some(rule) = queue.pop_front() {
                        if rule.ready_inputs() {
                            log::trace!("Add pending rule");
                            return Some(Box::pin(rule.process()));
                        } else {
                            log::trace!("Re-queue rule");
                            queue.push_back(rule);
                            out += 1;
                            if out >= queue.len() {
                                break;
                            }
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        while !opt_pending.is_empty() {
            log::trace!("Rules {} queued {} pending", queue.len(), opt_pending.len());
            let (result, _, mut pending) = future::select_all(opt_pending).await;
            if let Err(error) = result {
                log::error!("Rule invoking error: {}", error);
            }
            let mut out = 0;
            while !queue.is_empty() && pending.len() < jobs {
                log::trace!("Prepare pending rule");
                if let Some(rule) = queue.pop_front() {
                    if rule.ready_inputs() {
                        log::trace!("Add pending rule");
                        pending.push(Box::pin(rule.process()));
                    } else {
                        log::trace!("Re-queue rule");
                        queue.push_back(rule);
                        out += 1;
                        if out >= queue.len() {
                            break;
                        }
                    }
                }
            }
            opt_pending = pending;
        }

        if queue.is_empty() {
            Ok(())
        } else {
            log::warn!("Rules {} queued", queue.len());
            Err(format!("Cannot be built").into())
        }
    }

    pub async fn process<F>(&self, jobs: usize, matcher: F) -> Result<()>
    where
        F: Fn(&str) -> bool,
    {
        self.remove_expired();

        log::debug!("Init artifacts");
        self.init_artifacts().await?;

        log::debug!("Process artifacts");
        self.process_artifacts(
            jobs,
            self.phony
                .read()
                .iter()
                .filter(|artifact| matcher(artifact.name())),
        )
        .await?;

        log::debug!("Done");
        Ok(())
    }

    pub async fn update_source(&self, name: &str, time: Option<Time>) -> Result<bool> {
        if let Some(artifact) = self.actual.read().get(name) {
            if artifact.is_source() {
                let updated = artifact.update_time(time).await;
                log::trace!("Updated source {}", name);
                return updated;
            }
        }
        Ok(false)
    }

    pub async fn update_sources(
        &self,
        entries: impl IntoIterator<Item = (&str, Option<Time>)>,
    ) -> Result<bool> {
        future::join_all(
            entries
                .into_iter()
                .map(|(name, time)| self.update_source(name, time)),
        )
        .await
        .into_iter()
        .fold(Ok(false), |pre, cur| {
            pre.and_then(|pre_res| cur.map(|cur_res| pre_res || cur_res))
        })
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
