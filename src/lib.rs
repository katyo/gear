mod artifact;
mod compiler;
mod console;
mod diagnostic;
mod directory;
mod extensions;
mod hasher;
mod processor;
mod refs;
mod result;
mod rule;
mod scope;
mod store;
pub mod system;
mod utils;
mod variable;

pub use refs::*;
pub use result::*;
pub use utils::*;

pub use rquickjs as qjs;
pub use std::time::{Duration, SystemTime as Time};
pub use weak_table::traits::{WeakElement, WeakKey};

pub use artifact::{Actual, Artifact, ArtifactStore, Input, Output, Phony, WeakArtifact};
pub use diagnostic::{
    Diagnostic, Diagnostics, FixingSuggestion, Location, Severity, TextPoint, TextSpan,
};
pub use directory::Directory;
pub use hasher::DataHasher;
pub use processor::RuleStateChange;
pub use rule::{JsRule, NoRule, Rule, RuleApi, RuleId, RuleState};
pub use scope::Scope;
pub use store::Store;
pub use variable::{
    Value, ValueDef, ValueError, ValueResult, ValueStore, Variable, VariableDef, VariableStore,
    WeakVariable, WeakVariableSet,
};

pub use console::Js as ConsoleJs;
pub use extensions::Js as ExtensionsJs;
pub use system::Js as SystemJs;

pub use artifact::Js as ArtifactJs;
pub use directory::Js as DirectoryJs;
pub use rule::Js as RuleJs;
pub use scope::Js as ScopeJs;
pub use variable::Js as VariableJs;

pub use compiler::{CompilerJs, SymbolInfo, SymbolsJs};

use futures::future::LocalBoxFuture;
use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};
use weak_table::WeakHashSet;

pub type Set<T> = IndexSet<T, FxBuildHasher>;
pub type Map<K, V> = IndexMap<K, V, FxBuildHasher>;
pub type WeakSet<T> = WeakHashSet<T, FxBuildHasher>;
pub type BoxedFuture<T> = LocalBoxFuture<'static, T>;
