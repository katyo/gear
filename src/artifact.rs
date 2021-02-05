use crate::system::{access, modified, AccessMode, Path};
use crate::{
    qjs, Mut, Ref, Result, Rule, RuleState, Set, Time, Weak, WeakElement, WeakKey, WeakSet,
};
use derive_deref::Deref;
use either::{Left, Right};
use std::{
    borrow::Borrow,
    collections::VecDeque,
    fmt,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    iter::{empty, once},
    marker::PhantomData,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, qjs::FromJs, qjs::IntoJs)]
#[repr(u8)]
pub enum ArtifactType {
    Source,
    Product,
}

impl Display for ArtifactType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Source => "source",
            Self::Product => "product",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, qjs::FromJs, qjs::IntoJs)]
#[repr(u8)]
pub enum ArtifactKind {
    Actual,
    Phony,
}

impl Display for ArtifactKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Actual => "actual",
            Self::Phony => "phony",
        }
        .fmt(f)
    }
}

pub struct Internal {
    name: String,
    description: String,
    rule: Mut<Option<Rule>>,
    time: Mut<Time>,
    type_: ArtifactType,
    kind: ArtifactKind,
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
    }
}

pub trait IsArtifactUsage {
    const NAME: &'static str;
    const TYPE: ArtifactType;

    fn reusable<U, K>(artifact: &Artifact<U, K>) -> bool;
}

pub struct Input;

impl IsArtifactUsage for Input {
    const NAME: &'static str = "input";
    const TYPE: ArtifactType = ArtifactType::Source;

    fn reusable<U, K>(_artifact: &Artifact<U, K>) -> bool {
        true
    }
}

pub struct Output;

impl IsArtifactUsage for Output {
    const NAME: &'static str = "output";
    const TYPE: ArtifactType = ArtifactType::Product;

    fn reusable<U, K>(artifact: &Artifact<U, K>) -> bool {
        artifact.type_() != ArtifactType::Source && !artifact.has_rule()
    }
}

pub trait IsArtifactKind {
    const NAME: &'static str;
    const KIND: ArtifactKind;

    fn get_store(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>>
    where
        Self: Sized;
}

pub struct Actual;

impl IsArtifactKind for Actual {
    const NAME: &'static str = "actual";
    const KIND: ArtifactKind = ArtifactKind::Actual;

    fn get_store(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>> {
        &store.actual
    }
}

pub struct Phony;

impl IsArtifactKind for Phony {
    const NAME: &'static str = "phony";
    const KIND: ArtifactKind = ArtifactKind::Phony;

    fn get_store(store: &ArtifactStore) -> &Mut<WeakSet<WeakArtifact<(), Self>>> {
        &store.phony
    }
}

#[repr(transparent)]
pub struct Artifact<U = (), K = ()>(Ref<Internal>, PhantomData<(U, K)>);

impl<U, K> Display for Artifact<U, K> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Artifact(`{}`, {}, {}{})",
            self.name(),
            self.type_(),
            self.kind(),
            if self.has_rule() { ", rule" } else { "" }
        )
    }
}

impl<U, K> fmt::Debug for Artifact<U, K> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Display::fmt(self, f)
    }
}

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
    fn new_raw(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let description = description.into();
        log::debug!("Artifact::new `{}`", name);
        Self(
            Ref::new(Internal {
                name,
                description,
                rule: Default::default(),
                time: Mut::new(Time::UNIX_EPOCH),
                type_: U::TYPE,
                kind: K::KIND,
            }),
            PhantomData,
        )
    }

    pub fn new(
        set: impl AsRef<ArtifactStore>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self> {
        let set = K::get_store(set.as_ref());
        let name = name.into();
        let description = description.into();
        {
            // try reuse already existing artifact
            if let Some(artifact) = set.read().get(&name) {
                return artifact.into_usage();
            }
        }
        let artifact = Self::new_raw(name, description);
        set.write().insert(artifact.clone().into_usage_any());
        Ok(artifact)
    }
}

impl Artifact<Input, Actual> {
    pub async fn new_init(
        set: impl AsRef<ArtifactStore>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self> {
        let artifact = Self::new(set, name.into(), description.into())?;
        artifact.init().await?;
        Ok(artifact)
    }

    pub async fn init(&self) -> Result<()> {
        let path = Path::new(self.name());
        if !access(path, AccessMode::READ).await {
            return Err(format!("Unable to read input file `{}`", self.name()).into());
        }
        let time = modified(path).await?;
        self.set_time(time);
        Ok(())
    }
}

impl Artifact<Output, Actual> {
    pub async fn new_init(
        set: impl AsRef<ArtifactStore>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self> {
        let artifact = Self::new(set, name.into(), description.into())?;
        artifact.init().await?;
        Ok(artifact)
    }

    pub async fn init(&self) -> Result<()> {
        let path = Path::new(self.name());
        if path.exists().await {
            if !access(path, AccessMode::WRITE).await {
                return Err(format!("Unable to write output file `{}`", self.name()).into());
            }
            let time = modified(path).await?;
            self.set_time(time);
        }
        Ok(())
    }
}

impl<U, K> Artifact<U, K> {
    pub fn into_usage<T: IsArtifactUsage>(self) -> Result<Artifact<T, K>> {
        if T::reusable(&self) {
            Ok(Artifact(self.0, PhantomData))
        } else {
            Err(format!("Attempt to reuse {} as {}", self, T::NAME).into())
        }
    }

    pub fn into_usage_any(self) -> Artifact<(), K> {
        Artifact(self.0, PhantomData)
    }

    pub fn into_kind<T: IsArtifactKind>(self) -> Result<Artifact<U, T>> {
        if T::KIND == self.0.kind {
            Ok(Artifact(self.0, PhantomData))
        } else {
            Err(format!("Attempt to use {} as {}", self, T::KIND).into())
        }
    }

    pub fn into_kind_any(self) -> Artifact<U, ()> {
        Artifact(self.0, PhantomData)
    }

    pub fn ctor() -> Self {
        unimplemented!()
    }

    pub fn name(&self) -> &String {
        &self.0.name
    }

    pub fn description(&self) -> &String {
        &self.0.description
    }

    pub fn type_(&self) -> ArtifactType {
        self.0.type_
    }

    pub fn kind(&self) -> ArtifactKind {
        self.0.kind
    }

    pub fn time(&self) -> Time {
        *self.0.time.read()
    }

    pub fn has_rule(&self) -> bool {
        self.0.rule.read().is_some()
    }

    pub fn rule(&self) -> Option<Rule> {
        self.0.rule.read().clone()
    }

    pub fn weak(&self) -> WeakArtifact<U, K> {
        WeakArtifact(Ref::downgrade(&self.0), PhantomData)
    }

    pub fn is_source(&self) -> bool {
        self.type_() == ArtifactType::Source
    }

    pub fn is_phony(&self) -> bool {
        self.kind() == ArtifactKind::Phony
    }

    pub fn inputs(&self) -> impl Iterator<Item = Artifact<Input>> {
        self.0
            .rule
            .read()
            .as_ref()
            .map(|rule| Right(rule.inputs().into_iter()))
            .unwrap_or(Left(empty()))
    }

    pub fn state(&self) -> RuleState {
        let rule = self.0.rule.read();
        rule.as_ref().map(|rule| rule.state()).unwrap_or_default()
    }

    pub fn outdated(&self) -> bool {
        if self.is_source() {
            false
        } else {
            self.inputs()
                .any(|dep| dep.outdated() || dep.time() > self.time())
        }
    }

    pub fn set_time(&self, time: Time) {
        *self.0.time.write() = time;
    }

    pub async fn update_time(&self, new_time: Option<Time>) -> Result<bool> {
        let cur_time = modified(Path::new(self.name())).await?;
        Ok(if cur_time > self.time() {
            self.set_time(new_time.unwrap_or(cur_time));
            true
        } else {
            false
        })
    }

    pub fn fmt_tree(&self, ident: usize, f: &mut Formatter) -> FmtResult {
        let spaces = ident * 4;
        write!(f, "{:ident$}{}", "", self.name(), ident = spaces)?;
        let text = self.description();
        if !text.is_empty() {
            " // ".fmt(f)?;
            text.fmt(f)?;
        }
        '\n'.fmt(f)?;
        Ok(())
    }

    fn fmt_node_name(&self, f: &mut Formatter) -> FmtResult {
        f.write_fmt(format_args!("{:?}", self.name()))
    }

    pub fn fmt_dot_edges(&self, ident: usize, f: &mut Formatter) -> FmtResult {
        if !self.is_source() {
            let mut deps = self.inputs();
            if let Some(dep1) = deps.next() {
                let spaces = ident * 4;
                f.write_fmt(format_args!("{:ident$}", "", ident = spaces))?;
                if let Some(dep2) = deps.next() {
                    '{'.fmt(f)?;
                    dep1.fmt_node_name(f)?;
                    ' '.fmt(f)?;
                    dep2.fmt_node_name(f)?;
                    for dep in deps {
                        ' '.fmt(f)?;
                        dep.fmt_node_name(f)?;
                    }
                    '}'.fmt(f)?;
                } else {
                    dep1.fmt_node_name(f)?;
                }
                " -> ".fmt(f)?;
                self.fmt_node_name(f)?;
                if self.is_phony() {
                    " [style=dashed]".fmt(f)?;
                }
                ";\n".fmt(f)?;
            }
        }
        Ok(())
    }
    pub fn fmt_dot_node(&self, ident: usize, f: &mut Formatter) -> FmtResult {
        let spaces = ident * 4;
        f.write_fmt(format_args!(
            "{:ident$}{:?} [style=filled fillcolor={}];\n",
            "",
            self.name(),
            if self.is_source() {
                "aquamarine"
            } else if self.is_phony() {
                "goldenrod1"
            } else {
                "pink"
            },
            ident = spaces,
        ))?;
        Ok(())
    }

    pub fn process(&self, schedule: &mut impl FnMut(Rule)) -> bool {
        if self.is_source() {
            false
        } else if self
            .inputs()
            .map(|dep| dep.process(schedule) || dep.time() > self.time())
            .fold(self.is_phony(), |pre, flag| pre || flag)
        {
            self.schedule_rule(schedule);
            true
        } else {
            false
        }
    }

    fn schedule_rule(&self, schedule: &mut impl FnMut(Rule)) {
        if let Some(rule) = &*self.0.rule.read() {
            log::trace!("Schedule rule for `{}`", self.name());
            schedule(rule.clone());
        }
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

impl<K> Artifact<Output, K> {
    pub fn input(&self) -> Artifact<Input, K> {
        Artifact(self.0.clone(), PhantomData)
    }

    pub fn set_rule(&self, rule: impl Into<Rule>) {
        *self.0.rule.write() = Some(rule.into());
    }
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

pub type ArtifactWeakSet<K> = WeakSet<WeakArtifact<(), K>>;

#[derive(Default)]
pub struct StoreInternal {
    pub actual: Mut<ArtifactWeakSet<Actual>>,
    pub phony: Mut<ArtifactWeakSet<Phony>>,
}

#[derive(Default, Clone, Deref)]
pub struct ArtifactStore(Ref<StoreInternal>);

impl ArtifactStore {
    pub fn reset(&self) {
        *self.0.actual.write() = Default::default();
        *self.0.phony.write() = Default::default();
    }

    pub fn fmt_dot<F>(&self, matcher: F, f: &mut Formatter) -> FmtResult
    where
        F: Fn(&str) -> bool,
    {
        let mut queue: VecDeque<Vec<Artifact<Input>>> = {
            once(
                self.phony
                    .read()
                    .iter()
                    .filter(|artifact| matcher(artifact.name()))
                    .map(|a| a.into_kind_any().into_usage::<Input>().unwrap())
                    .collect(),
            )
            .collect()
        };
        let mut shown = Set::<Artifact<Input>>::default();

        "digraph {\n".fmt(f)?;
        loop {
            if let Some(artifacts) = queue.pop_front() {
                for artifact in artifacts {
                    if !shown.contains(&artifact) {
                        artifact.fmt_dot_edges(1, f)?;
                        let deps = artifact.inputs().collect::<Vec<_>>();
                        if !deps.is_empty() {
                            queue.push_back(deps);
                        }
                        shown.insert(artifact);
                    }
                }
            } else {
                break;
            }
        }
        for artifact in shown {
            artifact.fmt_dot_node(1, f)?;
        }
        "}\n".fmt(f)?;
        Ok(())
    }
}

impl AsRef<ArtifactStore> for ArtifactStore {
    fn as_ref(&self) -> &ArtifactStore {
        &*self
    }
}

impl<'js, U: IsArtifactUsage> qjs::FromJs<'js> for Artifact<U> {
    fn from_js(ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let artifact: Artifact = qjs::FromJs::from_js(ctx, val)?;

        if U::reusable(&artifact) {
            Ok(Artifact(artifact.0, PhantomData))
        } else {
            Err(qjs::Error::new_from_js("artifact", U::NAME))
        }
    }
}

impl<'js, K: IsArtifactKind> qjs::FromJs<'js> for Artifact<(), K> {
    fn from_js(ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let artifact: Artifact = qjs::FromJs::from_js(ctx, val)?;

        if K::KIND == artifact.kind() {
            Ok(Artifact(artifact.0, PhantomData))
        } else {
            Err(qjs::Error::new_from_js("artifact", K::NAME))
        }
    }
}

impl<'js, U: IsArtifactUsage, K: IsArtifactKind> qjs::FromJs<'js> for Artifact<U, K> {
    fn from_js(ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let artifact: Artifact<U> = qjs::FromJs::from_js(ctx, val)?;

        if K::KIND == artifact.kind() {
            Ok(Artifact(artifact.0, PhantomData))
        } else {
            Err(qjs::Error::new_from_js("artifact", K::NAME))
        }
    }
}

impl<'js, U: IsArtifactUsage> qjs::IntoJs<'js> for Artifact<U> {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.into_usage_any().into_js(ctx)
    }
}

impl<'js, K: IsArtifactKind> qjs::IntoJs<'js> for Artifact<(), K> {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.into_kind_any().into_js(ctx)
    }
}

impl<'js, U: IsArtifactUsage, K: IsArtifactKind> qjs::IntoJs<'js> for Artifact<U, K> {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.into_kind_any().into_usage_any().into_js(ctx)
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    pub type AnyArtifact = Artifact;

    #[quickjs(rename = "Artifact", cloneable)]
    impl AnyArtifact {
        #[quickjs(rename = "new", hide)]
        pub fn ctor() -> Self {}

        #[quickjs(get, enumerable, hide)]
        pub fn name(&self) -> &String {}

        #[quickjs(get, enumerable, hide, rename = "type")]
        pub fn type_(&self) -> ArtifactType {}

        #[quickjs(get, enumerable, hide)]
        pub fn kind(&self) -> ArtifactKind {}

        #[quickjs(get, enumerable, hide)]
        pub fn description(&self) -> &String {}

        #[quickjs(get, enumerable, hide)]
        pub fn rule(&self) -> Option<Rule> {}

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }
}
