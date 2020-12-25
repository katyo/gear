use crate::{qjs, Artifact, Mut, Ref, Set, Weak, WeakElement, WeakKey, WeakSet};
use derive_deref::Deref;
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

pub struct Internal {
    name: String,
    description: Mut<String>,
    artifacts: Mut<Set<Artifact>>,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("Goal::drop `{}`", self.name);
    }
}

impl Borrow<str> for Internal {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl Borrow<String> for Internal {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl PartialEq for Internal {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Internal {}

impl Hash for Internal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Goal(Ref<Internal>);

impl Goal {
    pub fn new_raw<S: Into<String>>(name: S) -> Self {
        let name = name.into();
        log::debug!("Goal::new `{}`", name);
        Self(Ref::new(Internal {
            name,
            description: Mut::new(Default::default()),
            artifacts: Mut::new(Default::default()),
        }))
    }

    pub fn new<W: AsRef<WeakGoalSet>, N: Into<String>>(set: W, name: N) -> Self {
        let set = set.as_ref();
        let name = name.into();
        {
            // try reuse already existing goal
            if let Some(goal) = set.read().get(&name) {
                return goal;
            }
        }
        let goal = Self::new_raw(name);
        set.write().insert(goal.clone());
        goal
    }

    pub fn set_description<S: Into<String>>(&self, description: S) {
        *self.0.description.write() = description.into();
    }

    pub fn set_artifacts(&self, artifacts: &[Artifact]) {
        *self.0.artifacts.write() = artifacts.iter().cloned().collect();
    }

    pub fn clear_artifacts(&self) {
        *self.0.artifacts.write() = Default::default();
    }

    pub fn weak(&self) -> WeakGoal {
        WeakGoal(Ref::downgrade(&self.0))
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct WeakGoal(Weak<Internal>);

impl WeakGoal {
    pub fn try_ref(&self) -> Option<Goal> {
        self.0.upgrade().map(Goal)
    }
}

impl WeakKey for WeakGoal {
    type Key = Internal;

    fn with_key<F, R>(view: &Self::Strong, f: F) -> R
    where
        F: FnOnce(&Self::Key) -> R,
    {
        f(&view.0)
    }
}

impl WeakElement for WeakGoal {
    type Strong = Goal;

    fn new(view: &Self::Strong) -> Self {
        view.weak()
    }

    fn view(&self) -> Option<Self::Strong> {
        self.try_ref()
    }

    fn clone(view: &Self::Strong) -> Self::Strong {
        view.clone()
    }
}

#[derive(Clone, Default, Deref)]
pub struct WeakGoalSet(Ref<Mut<WeakSet<WeakGoal>>>);

impl AsRef<WeakGoalSet> for WeakGoalSet {
    fn as_ref(&self) -> &WeakGoalSet {
        &*self
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(rename = "Goal")]
    impl Goal {
        #[quickjs(rename = "new")]
        pub fn ctor() -> Self {
            unimplemented!()
        }

        #[quickjs(get)]
        pub fn name(&self) -> &String {
            &self.0.name
        }

        #[quickjs(get)]
        pub fn description(&self) -> String {
            self.0.description.read().clone()
        }

        #[quickjs(rename = "description", set)]
        pub fn set_description_js(&self, description: String) {
            self.set_description(description)
        }

        #[quickjs(get)]
        pub fn artifacts(&self) -> Vec<Artifact> {
            self.0.artifacts.read().iter().cloned().collect()
        }

        #[quickjs(rename = "artifacts", set)]
        pub fn set_artifacts_js(&self, artifacts: Vec<&Artifact>) {
            *self.0.artifacts.write() = artifacts.into_iter().cloned().collect();
        }

        /*pub fn insert_artifact(&self, artifact: &Artifact) {
            self.0.artifacts.write().insert(artifact.clone());
        }

        pub fn remove_artifact(&self, artifact: &Artifact) {
            self.0.artifacts.write().remove(artifact);
        }*/
    }
}
