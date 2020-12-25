use crate::{qjs, Actual, Artifact, ArtifactStore, Input, Output, Ref, Result};
use relative_path::{RelativePath, RelativePathBuf};

pub struct Internal {
    artifacts: ArtifactStore,
    path: RelativePathBuf,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Directory(Ref<Internal>);

impl AsRef<ArtifactStore> for Directory {
    fn as_ref(&self) -> &ArtifactStore {
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
    pub fn new<A: AsRef<ArtifactStore>, P: Into<RelativePathBuf>>(artifacts: A, path: P) -> Self {
        Self(Ref::new(Internal {
            artifacts: artifacts.as_ref().clone(),
            path: path.into(),
        }))
    }

    pub fn child<P: AsRef<RelativePath>>(&self, path: P) -> Self {
        Self::new(self, self.0.path.join(path))
    }

    pub fn input<P: AsRef<RelativePath>>(&self, name: P) -> Result<Artifact<Input, Actual>> {
        Artifact::new(self, self.0.path.join(name).to_string())
    }

    pub fn output<P: AsRef<RelativePath>>(&self, name: P) -> Result<Artifact<Output, Actual>> {
        Artifact::new(self, self.0.path.join(name).to_string())
        /*let path = self.0.path.join(name).to_string();
        let artifact = Artifact::new(self, &path);
        if artifact.has_builder() {
            return Err(format!("Output artifact already exists `{}`", path).into());
        }
        Ok(artifact)*/
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
        pub fn _child(&self, path: String) -> Self {
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
        pub fn _input(&self, name: String) -> Result<Artifact<Input, Actual>> {
            self.input(name)
        }

        #[quickjs(rename = "output")]
        pub fn _output(&self, name: String) -> Result<Artifact<Output, Actual>> {
            self.output(name)
        }
    }
}
