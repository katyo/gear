use crate::{Artifact, ArtifactStore, Scope};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub struct NodeDisplay<T>(pub T);

impl<F> Display for NodeDisplay<(&Scope, &F)>
where
    F: Fn(&str) -> bool,
{
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let (scope, matcher) = self.0;
        scope.fmt_tree(0, matcher, fmt)
    }
}

impl<F> Display for NodeDisplay<(usize, &Scope, &F)>
where
    F: Fn(&str) -> bool,
{
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let (ident, scope, matcher) = self.0;
        scope.fmt_tree(ident, matcher, fmt)
    }
}

impl<F> Display for NodeDisplay<(&ArtifactStore, &F)>
where
    F: Fn(&str) -> bool,
{
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let (store, matcher) = self.0;
        store.fmt_dot(matcher, fmt)
    }
}

impl<U, K> Display for NodeDisplay<(usize, &Artifact<U, K>)> {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let (ident, artifact) = self.0;
        artifact.fmt_tree(ident, fmt)
    }
}
