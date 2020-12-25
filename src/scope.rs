use crate::{
    qjs, AnyKind, Artifact, ArtifactStore, Input, JsRule, Mut, NoRule, Output, Phony, Ref, Result,
    Set,
};
use derive_deref::Deref;
use either::Either;
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
    iter::once,
};

pub struct Internal {
    artifacts: ArtifactStore,
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

impl AsRef<ArtifactStore> for Scope {
    fn as_ref(&self) -> &ArtifactStore {
        &self.0.artifacts
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
    pub fn new<A, N>(artifacts: A, name: N) -> Self
    where
        A: AsRef<ArtifactStore>,
        N: Into<String>,
    {
        let name = name.into();
        log::debug!("Scope::new `{}`", name);
        Self(Ref::new(Internal {
            artifacts: artifacts.as_ref().clone(),
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

    pub fn input<N: AsRef<str>>(&self, name: N) -> Result<Artifact<Input, Phony>> {
        Artifact::new(self, join_name(self, name))
    }

    pub fn output<N: AsRef<str>>(&self, name: N) -> Result<Artifact<Output, Phony>> {
        Artifact::new(self, join_name(self, name))
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

#[derive(Clone, Deref)]
pub struct Goal<R>(R);

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

        #[quickjs(rename = "input")]
        pub fn input_js(&self, name: String) -> Result<Artifact<Input, Phony>> {
            self.input(name)
        }

        #[quickjs(rename = "output")]
        pub fn output_js(&self, name: String) -> Result<Artifact<Output, Phony>> {
            self.output(name)
        }

        #[quickjs(rename = "goal")]
        pub fn goal_fn<'js>(
            &self,
            name: String,
            ctx: qjs::Ctx<'js>,
            function: qjs::Persistent<qjs::Function<'static>>,
        ) -> Result<Goal<JsRule>> {
            let context = qjs::Context::from_ctx(ctx)?;
            self.output(name).map(|output| {
                Goal(JsRule::new_raw(
                    Default::default(),
                    once(output.into_kind_any()).collect(),
                    function,
                    context,
                ))
            })
        }

        pub fn goal(&self, name: String) -> Result<Goal<NoRule>> {
            self.output(name).map(|output| {
                Goal(NoRule::new_raw(
                    Default::default(),
                    once(output.into_kind_any()).collect(),
                ))
            })
        }
    }

    pub type NoRuleGoal = Goal<NoRule>;

    impl NoRuleGoal {
        #[quickjs(get)]
        pub fn input(&self) -> Option<Artifact<Input>> {
            self.0
                .outputs()
                .into_iter()
                .next()
                .map(|output| output.input())
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(
            &self,
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        ) {
            self.0.set_inputs(inputs)
        }
    }

    pub type JsRuleGoal = Goal<JsRule>;

    impl JsRuleGoal {
        #[quickjs(get)]
        pub fn input(&self) -> Option<Artifact<Input>> {
            self.0
                .outputs()
                .into_iter()
                .next()
                .map(|output| output.input())
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(
            &self,
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        ) {
            self.0.set_inputs(inputs)
        }
    }
}
