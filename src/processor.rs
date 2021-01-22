use crate::{Artifact, ArtifactStore, Result, Rule, RuleState, Set, Time};
use futures::future;
use std::{collections::VecDeque, future::Future, iter::once};

/// Changing rule state event
#[derive(Clone)]
pub struct RuleStateChange {
    pub rule: Rule,
    pub state: RuleState,
}

impl RuleStateChange {
    pub fn new(rule: Rule, state: RuleState) -> Self {
        Self { rule, state }
    }
}

impl ArtifactStore {
    async fn notify_rules_state<F, R, I>(emit: F, rules: I, state: RuleState)
    where
        F: Fn(RuleStateChange) -> R,
        R: Future<Output = ()>,
        I: Iterator<Item = Rule>,
    {
        for rule in rules {
            emit(RuleStateChange::new(rule, state)).await;
        }
    }

    async fn process_rule<F, R>(rule: Rule, emit: F) -> Result<()>
    where
        F: Fn(RuleStateChange) -> R + Clone,
        R: Future<Output = ()>,
    {
        let emit = emit.clone();

        Self::notify_rules_state(&emit, once(rule.clone()), RuleState::Processing).await;
        let result = rule.process().await;
        Self::notify_rules_state(&emit, once(rule.clone()), RuleState::Processed).await;
        result
    }

    async fn process_artifacts<K, I, F, R>(
        &self,
        artifacts: I,
        jobs: usize,
        dry_run: bool,
        emit: F,
    ) -> Result<()>
    where
        I: Iterator<Item = Artifact<(), K>>,
        F: Fn(RuleStateChange) -> R + Clone,
        R: Future<Output = ()>,
    {
        let mut queue = VecDeque::new();
        let mut unique = Set::default();
        let mut schedule = |rule: Rule| {
            let id = rule.id();
            if !unique.contains(&id) {
                unique.insert(id);
                rule.schedule();
                queue.push_back(rule);
            }
        };
        for artifact in artifacts {
            artifact.process(&mut schedule);
        }
        if dry_run {
            return Ok(());
        }

        Self::notify_rules_state(&emit, queue.iter().cloned(), RuleState::Scheduled).await;

        log::trace!("Prepare pending");
        let mut pending_tasks = (0..jobs)
            .into_iter()
            .filter_map(|_| {
                log::trace!("Prepare pending rule");
                let mut out = 0;
                while !queue.is_empty() {
                    if let Some(rule) = queue.pop_front() {
                        if rule.ready_inputs() {
                            log::trace!("Add pending rule");
                            return Some(Box::pin(Self::process_rule(rule, &emit)));
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

        while !pending_tasks.is_empty() {
            log::trace!(
                "Rules {} queued {} pending",
                queue.len(),
                pending_tasks.len()
            );
            let (result, _, mut pending) = future::select_all(pending_tasks).await;
            if let Err(error) = result {
                log::error!("Rule invoking error: {}", error);
            }
            let mut out = 0;
            while !queue.is_empty() && pending.len() < jobs {
                log::trace!("Prepare pending rule");
                if let Some(rule) = queue.pop_front() {
                    if rule.ready_inputs() {
                        log::trace!("Add pending rule");
                        pending.push(Box::pin(Self::process_rule(rule, &emit)));
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
            pending_tasks = pending;
        }

        if queue.is_empty() {
            Ok(())
        } else {
            log::warn!("Rules {} queued", queue.len());
            Err(format!("Cannot be built").into())
        }
    }

    pub async fn process<S, I, F, R>(
        &self,
        goals: I,
        jobs: usize,
        dry_run: bool,
        emit: F,
    ) -> Result<()>
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
        F: Fn(RuleStateChange) -> R + Clone,
        R: Future<Output = ()>,
    {
        log::debug!("Process artifacts");
        self.process_artifacts(
            goals
                .into_iter()
                .filter_map(|name| self.phony.read().get(name.as_ref())),
            jobs,
            dry_run,
            emit,
        )
        .await?;

        self.remove_expired();

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
}
