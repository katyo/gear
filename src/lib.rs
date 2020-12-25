mod artifact;
mod builder;
mod common;
mod compiler;
mod console;
mod directory;
mod goal;
mod refs;
mod result;
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

pub use artifact::{Artifact, WeakArtifact, WeakArtifactSet};
pub use builder::{Builder, BuilderApi};
pub use directory::Directory;
pub use goal::{Goal, WeakGoal, WeakGoalSet};
pub use scope::Scope;

pub type Set<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;
pub type Map<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;

pub use artifact::Js as ArtifactJs;
pub use builder::Js as BuilderJs;
pub use directory::Js as DirectoryJs;
pub use goal::Js as GoalJs;
pub use scope::Js as ScopeJs;

pub use compiler::GccJs;
