mod artifact;
mod common;
mod compiler;
mod console;
mod directory;
mod refs;
mod result;
mod rule;
mod scope;
mod system;
//mod variable;

pub use common::*;
pub use refs::*;
pub use result::*;

pub use rquickjs as qjs;

pub use console::Js as ConsoleJs;
pub use system::Js as SystemJs;

pub use std::time::{Duration, SystemTime};

pub use artifact::{Actual, AnyKind, Artifact, ArtifactStore, Input, Output, Phony, WeakArtifact};
pub use directory::Directory;
pub use rule::{JsRule, NoRule, Rule, RuleApi};
pub use scope::Scope;

pub type Set<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;
pub type Map<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;

pub use artifact::Js as ArtifactJs;
pub use directory::Js as DirectoryJs;
pub use rule::Js as RuleJs;
pub use scope::Js as ScopeJs;

pub use compiler::GccJs;
