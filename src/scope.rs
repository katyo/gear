use crate::{
    qjs, AnyKind, Artifact, ArtifactStore, Input, JsRule, Mut, NoRule, Output, Phony, Ref, Result,
    Set, Store, Value, ValueDef, Variable, VariableStore, WeakVariableSet,
};
use derive_deref::Deref;
use either::Either;
use std::{
    borrow::Borrow,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    iter::once,
};

pub struct Internal {
    store: Store,
    name: String,
    description: String,
    scopes: Mut<Set<Scope>>,
    variables: Mut<Set<Variable>>,
    goals: Mut<Set<Artifact<Output, Phony>>>,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("Scope::drop `{}`", self.name);
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Scope(Ref<Internal>);

impl AsRef<VariableStore> for Scope {
    fn as_ref(&self) -> &VariableStore {
        &self.0.store.as_ref()
    }
}

impl AsRef<ArtifactStore> for Scope {
    fn as_ref(&self) -> &ArtifactStore {
        &self.0.store.as_ref()
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

impl Display for Scope {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "Scope `".fmt(f)?;
        self.name().fmt(f)?;
        '`'.fmt(f)
    }
}

impl Scope {
    /// Create new scope
    pub fn new<N, D>(store: Store, name: N, description: D) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        let name = name.into();
        log::debug!("Scope::new `{}`", name);
        Self(Ref::new(Internal {
            store,
            name,
            description: description.into(),
            scopes: Default::default(),
            variables: Default::default(),
            goals: Default::default(),
        }))
    }

    /// Create new root scope
    pub fn new_root(store: Store) -> Self {
        Self::new(store, "", "")
    }

    /// Reset this scope to default
    ///
    /// This function removes all sub-scopes, goals, variables and artifacts.
    pub fn reset(&self) {
        self.0.store.reset();
        *self.0.scopes.write() = Default::default();
        *self.0.goals.write() = Default::default();
    }

    /// Get sub-scopes of this scope
    pub fn scopes(&self) -> Vec<Scope> {
        self.0.scopes.read().iter().cloned().collect::<Vec<_>>()
    }

    /// Get sub-scope by name
    pub fn scope<N: AsRef<str>>(&self, name: N) -> Option<Self> {
        self.0
            .scopes
            .read()
            .get(&self.full_name(name))
            .map(Self::clone)
    }

    /// Create new sub-scope in this scope
    pub fn new_scope(&self, name: impl AsRef<str>, description: impl Into<String>) -> Result<Self> {
        let name = self.full_name(name);
        {
            if self.0.scopes.read().contains(&name) {
                return Err(format!("Scope `{}` already exists", name).into());
            }
        }

        let scope = Self::new(self.0.store.clone(), name, description);
        self.0.scopes.write().insert(scope.clone());
        Ok(scope)
    }

    /// Get variables of this scope
    pub fn vars(&self) -> Vec<Variable> {
        self.0.variables.read().iter().cloned().collect::<Vec<_>>()
    }

    /// Get variable by name
    pub fn var(&self, name: impl AsRef<str>) -> Option<Variable> {
        self.0
            .variables
            .read()
            .get(&self.full_name(name))
            .map(Clone::clone)
    }

    /// Create new variable in this scope
    pub fn new_var(
        &self,
        name: impl AsRef<str>,
        description: impl Into<String>,
        definition: Option<ValueDef>,
        default: Option<Value>,
    ) -> Result<Variable> {
        let name = self.full_name(name);
        let variables: &VariableStore = self.0.store.as_ref();
        let variable = variables.new_variable(name, description, definition, default)?;
        self.0.variables.write().insert(variable.clone());
        Ok(variable)
    }

    /// Get goals of this scope
    pub fn goals(&self) -> Vec<Artifact<Output, Phony>> {
        self.0.goals.read().iter().cloned().collect::<Vec<_>>()
    }

    /// Get goal by name
    pub fn goal(&self, name: impl AsRef<str>) -> Option<Artifact<Output, Phony>> {
        self.0
            .goals
            .read()
            .get(&self.full_name(name))
            .map(Artifact::clone)
    }

    /// Create new goal in this scope
    pub fn new_goal(
        &self,
        name: impl AsRef<str>,
        description: impl AsRef<str>,
    ) -> Result<Artifact<Output, Phony>> {
        let goal = Artifact::new(self, self.full_name(name), description.as_ref())?;
        self.0.goals.write().insert(goal.clone());
        Ok(goal)
    }

    pub fn is_root(&self) -> bool {
        self.name().is_empty()
    }

    pub fn fmt_tree(
        &self,
        ident: usize,
        matcher: &impl Fn(&str) -> bool,
        f: &mut Formatter,
    ) -> FmtResult {
        let ident = if self.is_root() {
            ident
        } else {
            let spaces = ident * 4;
            write!(f, "{:ident$}{}", "", self.name(), ident = spaces)?;
            let text = self.description();
            if !text.is_empty() {
                " // ".fmt(f)?;
                text.fmt(f)?;
            }
            '\n'.fmt(f)?;
            ident + 1
        };

        for var in self.vars() {
            if matcher(&var.name()) {
                var.fmt_tree(ident, f)?;
            }
        }

        for goal in self.goals() {
            if matcher(&goal.name()) {
                goal.fmt_tree(ident, f)?;
            }
        }

        for scope in self.scopes() {
            if matcher(&scope.name()) {
                scope.fmt_tree(ident, matcher, f)?;
            }
        }
        Ok(())
    }

    fn full_name<N: AsRef<str>>(&self, name: N) -> String {
        let name = name.as_ref();
        if self.name().is_empty() {
            name.into()
        } else {
            [&self.name(), name].join(".")
        }
    }
}

#[derive(Clone, Deref)]
pub struct Goal<R>(R);

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Scope {
        #[doc(hidden)]
        #[quickjs(rename = "new")]
        pub fn ctor() -> Self {
            unimplemented!()
        }

        #[quickjs(get, enumerable)]
        pub fn name(&self) -> &String {
            &self.0.name
        }

        #[quickjs(get, enumerable)]
        pub fn description(&self) -> &String {
            &self.0.description
        }

        #[doc(hidden)]
        #[quickjs(rename = "scope")]
        pub fn scope_js0(&self, name: String, description: qjs::Opt<String>) -> Result<Self> {
            self.new_scope(&name, description.0.unwrap_or_default())
                .or_else(|error| self.scope(name).ok_or(error))
        }

        #[doc(hidden)]
        #[quickjs(rename = "var")]
        pub fn var_js1(
            &self,
            name: String,
            description: String,
            definition: ValueDef,
            default: qjs::Opt<Value>,
        ) -> Result<Variable> {
            self.new_var(name, description, Some(definition), default.0)
        }

        #[doc(hidden)]
        #[quickjs(rename = "var")]
        pub fn var_js0(&self, name: String) -> Option<Variable> {
            self.var(name)
        }

        /*#[doc(hidden)]
        #[quickjs(rename = "vars")]
        pub async fn vars_js3(self, name: String) -> Result<()> {}*/

        #[doc(hidden)]
        #[quickjs(rename = "goal")]
        pub fn goal_js1<'js>(
            &self,
            name: String,
            description: String,
            function: qjs::Persistent<qjs::Function<'static>>,
            ctx: qjs::Ctx<'js>,
        ) -> Result<Goal<JsRule>> {
            let context = qjs::Context::from_ctx(ctx)?;
            let artifact = self.new_goal(name, description)?;
            Ok(Goal(JsRule::new_raw(
                Default::default(),
                once(artifact.into_kind_any()).collect(),
                function,
                context,
            )))
        }

        #[doc(hidden)]
        #[quickjs(rename = "goal")]
        pub fn goal_js2(
            &self,
            name: String,
            description: qjs::Opt<String>,
        ) -> Result<Goal<NoRule>> {
            let artifact = self.new_goal(name, description.0.unwrap_or_default())?;
            Ok(Goal(NoRule::new_raw(
                Default::default(),
                once(artifact.into_kind_any()).collect(),
            )))
        }

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }

    pub type NoRuleGoal = Goal<NoRule>;

    impl NoRuleGoal {
        #[quickjs(get, enumerable)]
        pub fn input(&self) -> Option<Artifact<Input>> {
            self.0
                .outputs()
                .into_iter()
                .next()
                .map(|output| output.input())
        }

        #[quickjs(get, enumerable)]
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

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.0.to_string()
        }
    }

    pub type JsRuleGoal = Goal<JsRule>;

    impl JsRuleGoal {
        #[quickjs(get, enumerable)]
        pub fn input(&self) -> Option<Artifact<Input>> {
            self.0
                .outputs()
                .into_iter()
                .next()
                .map(|output| output.input())
        }

        #[quickjs(get, enumerable)]
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

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.0.to_string()
        }
    }
}
