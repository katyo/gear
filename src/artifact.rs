use crate::{qjs, Builder, Mut, Ref, SystemTime, Weak, WeakElement, WeakKey, WeakSet};
use derive_deref::Deref;
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

pub struct Internal {
    name: String,
    time: Mut<SystemTime>,
    builder: Mut<Option<Builder>>,
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

#[derive(Clone)]
#[repr(transparent)]
pub struct Artifact(Ref<Internal>);

impl Borrow<str> for Artifact {
    fn borrow(&self) -> &str {
        &self.0.name
    }
}

impl Borrow<String> for Artifact {
    fn borrow(&self) -> &String {
        &self.0.name
    }
}

impl PartialEq for Artifact {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Artifact {}

impl Hash for Artifact {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Artifact {
    fn new_raw<S: Into<String>>(name: S) -> Self {
        let name = name.into();
        log::debug!("Artifact::new `{}`", name);
        Self(Ref::new(Internal {
            name: name.into(),
            time: Mut::new(SystemTime::UNIX_EPOCH),
            builder: Mut::new(None),
        }))
    }

    pub fn new<A: AsRef<WeakArtifactSet>, S: Into<String>>(set: A, name: S) -> Self {
        let set = set.as_ref();
        let name = name.into();
        {
            // try reuse already existing artifact
            if let Some(artifact) = set.read().get(&name) {
                return artifact;
            }
        }
        let artifact = Self::new_raw(name);
        set.write().insert(artifact.clone());
        artifact
    }

    pub fn time(&self) -> SystemTime {
        *self.0.time.read()
    }

    pub fn set_time(&self, time: SystemTime) {
        *self.0.time.write() = time;
    }

    pub fn has_builder(&self) -> bool {
        self.0.builder.read().is_some()
    }

    pub fn set_builder(&self, builder: Builder) {
        *self.0.builder.write() = Some(builder);
    }

    pub fn clear_builder(&self) {
        *self.0.builder.write() = None;
    }

    pub fn weak(&self) -> WeakArtifact {
        WeakArtifact(Ref::downgrade(&self.0))
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct WeakArtifact(Weak<Internal>);

impl WeakArtifact {
    pub fn try_ref(&self) -> Option<Artifact> {
        self.0.upgrade().map(Artifact)
    }
}

impl WeakKey for WeakArtifact {
    type Key = Internal;

    fn with_key<F, R>(view: &Self::Strong, f: F) -> R
    where
        F: FnOnce(&Self::Key) -> R,
    {
        f(&view.0)
    }
}

impl WeakElement for WeakArtifact {
    type Strong = Artifact;

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

#[derive(Default, Clone, Deref)]
pub struct WeakArtifactSet(Ref<Mut<WeakSet<WeakArtifact>>>);

impl AsRef<WeakArtifactSet> for WeakArtifactSet {
    fn as_ref(&self) -> &WeakArtifactSet {
        &*self
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Artifact {
        #[quickjs(rename = "new")]
        pub fn ctor() -> Self {
            unimplemented!()
        }

        #[quickjs(get)]
        pub fn name(&self) -> &String {
            &self.0.name
        }

        #[quickjs(get)]
        pub fn builder(&self) -> Option<Builder> {
            self.0.builder.read().clone()
        }
    }
}
