use derive_deref::Deref;
use rhai::{Engine, RegisterFn};
use std::{
    iter::{once, Iterator},
    path::PathBuf,
    rc::Rc,
};

/// Artifact
#[derive(Clone, Deref)]
pub struct Artifact(Rc<ArtifactData>);

pub struct ArtifactData {
    path: PathBuf,
}

impl Artifact {
    /// Create artifact with path
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        Self(Rc::new(ArtifactData { path }))
    }

    pub fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<Self>("Artifact")
            .register_fn("Artifact", Self::new::<&str>);
    }
}

/// Artifacts
#[derive(Clone, Default, Deref)]
pub struct Artifacts(Rc<ArtifactsData>);

#[derive(Default)]
pub struct ArtifactsData {
    list: Vec<Artifact>,
}

impl Artifacts {
    /// Create artifacts with path
    pub fn new<I: IntoIterator<Item = Artifact>>(artifacts: I) -> Self {
        Self(Rc::new(ArtifactsData {
            list: artifacts.into_iter().collect(),
        }))
    }

    pub fn register(engine: &mut Engine) {
        engine.register_type::<Self>()
        //.register_fn("Artifacts", Self::new)
            ;
    }
}

impl From<Artifact> for Artifacts {
    fn from(artifact: Artifact) -> Self {
        Self::new(once(artifact))
    }
}
