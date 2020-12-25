use crate::{qjs, Goal, Mut, Ref, Set, WeakGoalSet};
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

pub struct Internal {
    goals: WeakGoalSet,
    scopes: Mut<Set<Scope>>,
    name: String,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("Scope::drop `{}`", self.name);
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Scope(Ref<Internal>);

impl AsRef<WeakGoalSet> for Scope {
    fn as_ref(&self) -> &WeakGoalSet {
        &self.0.goals
    }
}

impl AsRef<str> for Scope {
    fn as_ref(&self) -> &str {
        &self.0.name
    }
}

impl AsRef<String> for Scope {
    fn as_ref(&self) -> &String {
        &self.0.name
    }
}

impl Borrow<str> for Scope {
    fn borrow(&self) -> &str {
        &self.0.name
    }
}

impl Borrow<String> for Scope {
    fn borrow(&self) -> &String {
        &self.0.name
    }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for Scope {}

impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
    }
}

impl Scope {
    pub fn new<W: AsRef<WeakGoalSet>, N: Into<String>>(goals: W, name: N) -> Self {
        let name = name.into();
        log::debug!("Scope::new `{}`", name);
        Self(Ref::new(Internal {
            goals: goals.as_ref().clone(),
            scopes: Default::default(),
            name,
        }))
    }

    pub fn scope<N: AsRef<str>>(&self, name: N) -> Self {
        let name = name.as_ref();
        self.0
            .scopes
            .read()
            .get(name)
            .map(|scope| scope.clone())
            .unwrap_or_else(|| {
                let scope = Self::new(self, join_name(self, name));
                self.0.scopes.write().insert(scope.clone());
                scope
            })
    }

    pub fn goal<N: AsRef<str>>(&self, name: N) -> Goal {
        Goal::new(self, join_name(self, name))
    }
}

fn join_name<P: AsRef<str>, N: AsRef<str>>(parent: P, name: N) -> String {
    let parent = parent.as_ref();
    let name = name.as_ref();
    if parent.is_empty() {
        name.into()
    } else {
        [parent, name].join(".")
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Scope {
        #[quickjs(rename = "new")]
        pub fn ctor() -> Self {
            unimplemented!()
        }

        #[quickjs(get)]
        pub fn name(&self) -> &str {
            self.0.name.as_ref()
        }

        #[quickjs(rename = "scope")]
        pub fn scope_js(&self, name: String) -> Self {
            self.scope(name)
        }

        #[quickjs(rename = "goal")]
        pub fn goal_js(&self, name: String) -> Goal {
            self.goal(name)
        }
    }
}
