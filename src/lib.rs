mod artifact;
mod compiler;
mod console;
mod directory;
mod processor;
mod refs;
mod result;
mod rule;
mod scope;
pub mod system;
mod utils;
//mod variable;

pub use refs::*;
pub use result::*;
pub use utils::*;

pub use rquickjs as qjs;

pub use console::Js as ConsoleJs;
pub use system::Js as SystemJs;

pub use std::time::{Duration, SystemTime as Time};

pub use artifact::{Actual, AnyKind, Artifact, ArtifactStore, Input, Output, Phony, WeakArtifact};
pub use directory::Directory;
pub use rule::{JsRule, NoRule, Rule, RuleApi};
pub use scope::Scope;

pub use weak_table::traits::{WeakElement, WeakKey};

pub use artifact::Js as ArtifactJs;
pub use directory::Js as DirectoryJs;
pub use rule::Js as RuleJs;
pub use scope::Js as ScopeJs;

pub use compiler::GccJs;

use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};
use weak_table::WeakHashSet;

pub type Set<T> = IndexSet<T, FxBuildHasher>;
pub type Map<K, V> = IndexMap<K, V, FxBuildHasher>;
pub type WeakSet<T> = WeakHashSet<T, FxBuildHasher>;
