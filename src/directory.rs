use crate::{qjs, Artifact, Ref, Result, WeakArtifactSet};
use relative_path::{RelativePath, RelativePathBuf};

pub struct Internal {
    artifacts: WeakArtifactSet,
    path: RelativePathBuf,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Directory(Ref<Internal>);

impl AsRef<WeakArtifactSet> for Directory {
    fn as_ref(&self) -> &WeakArtifactSet {
        &self.0.artifacts
    }
}

impl AsRef<RelativePathBuf> for Directory {
    fn as_ref(&self) -> &RelativePathBuf {
        &self.0.path
    }
}

impl AsRef<RelativePath> for Directory {
    fn as_ref(&self) -> &RelativePath {
        &self.0.path
    }
}

impl Directory {
    pub fn new<A: AsRef<WeakArtifactSet>, P: Into<RelativePathBuf>>(artifacts: A, path: P) -> Self {
        Self(Ref::new(Internal {
            artifacts: artifacts.as_ref().clone(),
            path: path.into(),
        }))
    }

    pub fn child<P: AsRef<RelativePath>>(&self, path: P) -> Self {
        Self::new(self, self.0.path.join(path))
    }

    pub fn input<P: AsRef<RelativePath>>(&self, name: P) -> Artifact {
        Artifact::new(self, self.0.path.join(name).to_string())
    }

    pub fn output<P: AsRef<RelativePath>>(&self, name: P) -> Result<Artifact> {
        let path = self.0.path.join(name).to_string();
        let artifact = Artifact::new(self, &path);
        if artifact.has_builder() {
            return Err(format!("Output artifact already exists `{}`", path).into());
        }
        Ok(artifact)
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Directory {
        #[quickjs(rename = "new")]
        pub fn ctor() -> Self {
            unimplemented!()
        }

        #[quickjs(get)]
        pub fn path(&self) -> &str {
            self.0.path.as_ref()
        }

        #[quickjs(rename = "child")]
        pub fn child_js(&self, path: String) -> Self {
            self.child(path)
        }

        #[quickjs(get)]
        pub fn parent(&self) -> Option<Self> {
            self.0
                .path
                .parent()
                .map(|path| Self::new(self, path.to_owned()))
        }

        #[quickjs(rename = "input")]
        pub fn input_js(&self, name: String) -> Artifact {
            self.input(name)
        }

        #[quickjs(rename = "output")]
        pub fn output_js(&self, name: String) -> Result<Artifact> {
            self.output(name)
        }
    }
}
