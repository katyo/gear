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
    pub fn new(artifacts: impl AsRef<ArtifactStore>, path: impl Into<RelativePathBuf>) -> Self {
        Self(Ref::new(Internal {
            artifacts: artifacts.as_ref().clone(),
            path: path.into(),
        }))
    }

    pub fn child(&self, path: impl AsRef<RelativePath>) -> Self {
        Self::new(self, self.0.path.join(path))
    }

    pub async fn input(&self, name: impl AsRef<RelativePath>) -> Result<Artifact<Input, Actual>> {
        let artifact = Artifact::new(self, self.0.path.join(name).to_string(), "")?;
        artifact.init().await?;
        Ok(artifact)
    }

    pub async fn output(&self, name: impl AsRef<RelativePath>) -> Result<Artifact<Output, Actual>> {
        let artifact = Artifact::new(self, self.0.path.join(name).to_string(), "")?;
        artifact.init().await?;
        Ok(artifact)
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(cloneable)]
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

        #[doc(hidden)]
        #[quickjs(rename = "input")]
        pub async fn input_js(self, name: String) -> Result<Artifact<Input, Actual>> {
            self.input(name).await
        }

        #[doc(hidden)]
        #[quickjs(rename = "output")]
        pub async fn output_js(self, name: String) -> Result<Artifact<Output, Actual>> {
            self.output(name).await
        }
    }
}
