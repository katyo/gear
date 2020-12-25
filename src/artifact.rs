use crate::{qjs, Mut, Ref, Result, Rule, SystemTime, Weak, WeakElement, WeakKey, WeakSet};
use derive_deref::{Deref, DerefMut};
use std::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ArtifactKind {
    Actual,
    Phony,
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Actual => "actual",
            Self::Phony => "phony",
        }
        .fmt(f)
    }
}

pub struct Internal {
    name: String,
    kind: ArtifactKind,
    time: Mut<SystemTime>,
    rule: Mut<Option<Rule>>,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("Artifact::drop `{}`", self.name);
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
        self.kind.hash(state);
    }
}

pub trait IsArtifactUsage {
    const REUSABLE: bool;
}

pub struct Input;

impl IsArtifactUsage for Input {
    const REUSABLE: bool = true;
}

pub struct Output;

impl IsArtifactUsage for Output {
    const REUSABLE: bool = false;
}

pub trait IsArtifactKind {
    const KIND: ArtifactKind;
    fn set(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>>
    where
        Self: Sized;
}

pub struct Actual;

impl IsArtifactKind for Actual {
    const KIND: ArtifactKind = ArtifactKind::Actual;

    fn set(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>> {
        &store.actual
    }
}

pub struct Phony;

impl IsArtifactKind for Phony {
    const KIND: ArtifactKind = ArtifactKind::Phony;

    fn set(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>> {
        &store.phony
    }
}

#[repr(transparent)]
pub struct Artifact<U = (), K = ()>(Ref<Internal>, PhantomData<(U, K)>);

impl<U, K> Clone for Artifact<U, K> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<U, K> Borrow<str> for Artifact<U, K> {
    fn borrow(&self) -> &str {
        &self.0.name
    }
}

impl<U, K> Borrow<String> for Artifact<U, K> {
    fn borrow(&self) -> &String {
        &self.0.name
    }
}

impl<U, V, K> PartialEq<Artifact<V, K>> for Artifact<U, K> {
    fn eq(&self, other: &Artifact<V, K>) -> bool {
        self.0 == other.0
    }
}

impl<U, K> Eq for Artifact<U, K> {}

impl<U, K> Hash for Artifact<U, K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<U, K> Artifact<U, K>
where
    U: IsArtifactUsage,
    K: IsArtifactKind,
{
    fn new_raw<S: Into<String>>(name: S) -> Self {
        let name = name.into();
        log::debug!("Artifact::new `{}`", name);
        Self(
            Ref::new(Internal {
                name: name.into(),
                kind: K::KIND,
                time: Mut::new(SystemTime::UNIX_EPOCH),
                rule: Default::default(),
            }),
            PhantomData,
        )
    }

    pub fn new<A, S>(set: A, name: S) -> Result<Self>
    where
        A: AsRef<ArtifactStore>,
        S: Into<String>,
    {
        let set = K::set(set.as_ref());
        let name = name.into();
        {
            // try reuse already existing artifact
            if let Some(artifact) = set.read().get(&name) {
                return if U::REUSABLE {
                    Ok(artifact.into_usage())
                } else {
                    Err(format!("Artifact `{}` already exists.", name).into())
                };
            }
        }
        let artifact = Self::new_raw(name);
        set.write().insert(artifact.clone().into_usage());
        Ok(artifact)
    }
}

impl<U, K> Artifact<U, K> {
    pub fn into_usage<T>(self) -> Artifact<T, K> {
        Artifact(self.0, PhantomData)
    }

    pub fn ctor() -> Self {
        unimplemented!()
    }

    pub fn name(&self) -> &String {
        &self.0.name
    }

    pub fn time(&self) -> SystemTime {
        *self.0.time.read()
    }

    pub fn weak(&self) -> WeakArtifact<U, K> {
        WeakArtifact(Ref::downgrade(&self.0), PhantomData)
    }
}

impl<K> From<Artifact<Input, K>> for Artifact<(), K> {
    fn from(artifact: Artifact<Input, K>) -> Self {
        Artifact(artifact.0, PhantomData)
    }
}

impl<K> From<Artifact<Output, K>> for Artifact<(), K> {
    fn from(artifact: Artifact<Output, K>) -> Self {
        Artifact(artifact.0, PhantomData)
    }
}

impl<K> From<Artifact<(), K>> for Artifact<Input, K> {
    fn from(artifact: Artifact<(), K>) -> Self {
        Artifact(artifact.0, PhantomData)
    }
}

impl<K> From<Artifact<Output, K>> for Artifact<Input, K> {
    fn from(artifact: Artifact<Output, K>) -> Self {
        Artifact(artifact.0, PhantomData)
    }
}

impl<U, K> Artifact<U, K> {
    pub fn into_kind<T: IsArtifactKind>(self) -> Result<Artifact<U, T>> {
        if T::KIND == self.0.kind {
            Ok(Artifact(self.0, PhantomData))
        } else {
            Err(format!(
                "Required {} artifact but actual artifact `{}` is {}",
                T::KIND,
                self.0.name,
                self.0.kind
            )
            .into())
        }
    }

    pub fn into_kind_any(self) -> Artifact<U, ()> {
        Artifact(self.0, PhantomData)
    }
}

impl<K> Artifact<Output, K> {
    pub fn set_time(&self, time: SystemTime) {
        *self.0.time.write() = time;
    }

    pub fn rule(&self) -> Option<Rule> {
        self.0.rule.read().clone()
    }

    pub fn input(&self) -> Artifact<Input, K> {
        Artifact(self.0.clone(), PhantomData)
    }

    pub fn has_rule(&self) -> bool {
        self.0.rule.read().is_some()
    }

    pub fn set_rule(&self, rule: Rule) {
        *self.0.rule.write() = Some(rule);
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct AnyKind<A>(pub A);

macro_rules! any_kind {
	  ($($usage:ident $($kind:ident)*;)*) => {
		    $(
            impl<'js> qjs::FromJs<'js> for AnyKind<&Artifact<$usage>> {
                fn from_js(ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
                    <&Artifact<$usage>>::from_js(ctx, val.clone())
                        .map(AnyKind)
                        $(
                            .or_else(|error| {
                                if error.is_from_js() {
                                    <&Artifact<$usage, $kind>>::from_js(ctx, val.clone()).map(|this| {
                                    AnyKind(unsafe { &*(this as *const Artifact<$usage, $kind> as *const _) })
                                    })
                                } else {
                                    Err(error)
                                }
                            })
                        )*
                }
            }
        )*
	  };
}

any_kind! {
    Input Actual Phony;
    Output Actual Phony;
}

#[derive(Clone)]
#[repr(transparent)]
pub struct WeakArtifact<U = (), K = ()>(Weak<Internal>, PhantomData<(U, K)>);

impl<U, K> WeakArtifact<U, K> {
    pub fn try_ref(&self) -> Option<Artifact<U, K>> {
        self.0.upgrade().map(|raw| Artifact(raw, PhantomData))
    }
}

impl<U, K> WeakKey for WeakArtifact<U, K> {
    type Key = Internal;

    fn with_key<F, R>(view: &Self::Strong, f: F) -> R
    where
        F: FnOnce(&Self::Key) -> R,
    {
        f(&view.0)
    }
}

impl<U, K> WeakElement for WeakArtifact<U, K> {
    type Strong = Artifact<U, K>;

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

#[derive(Default)]
pub struct StoreInternal {
    actual: Mut<WeakSet<WeakArtifact<(), Actual>>>,
    phony: Mut<WeakSet<WeakArtifact<(), Phony>>>,
}

#[derive(Default, Clone, Deref)]
pub struct ArtifactStore(Ref<StoreInternal>);

impl AsRef<ArtifactStore> for ArtifactStore {
    fn as_ref(&self) -> &ArtifactStore {
        &*self
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    pub type GenericInput = Artifact<Input>;

    impl GenericInput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}
    }

    pub type GenericOutput = Artifact<Output>;

    impl GenericOutput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}

        #[quickjs(get, hide)]
        pub fn rule(&self) -> Option<Rule> {}

        #[quickjs(get, hide)]
        pub fn input(&self) -> Artifact<Input> {}
    }

    pub type ActualInput = Artifact<Input, Actual>;

    impl ActualInput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}
    }

    pub type ActualOutput = Artifact<Output, Actual>;

    impl ActualOutput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}

        #[quickjs(get, hide)]
        pub fn rule(&self) -> Option<Rule> {}

        #[quickjs(get, hide)]
        pub fn input(&self) -> Artifact<Input, Actual> {}
    }

    pub type PhonyInput = Artifact<Input, Phony>;

    impl PhonyInput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}
    }

    pub type PhonyOutput = Artifact<Output, Phony>;

    impl PhonyOutput {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, hide)]
        pub fn name(&self) -> &String {}

        #[quickjs(get, hide)]
        pub fn rule(&self) -> Option<Rule> {}

        #[quickjs(get, hide)]
        pub fn input(&self) -> Artifact<Input, Phony> {}
    }
}
