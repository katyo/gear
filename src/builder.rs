use derive_deref::Deref;
use rhai::{Engine, RegisterFn};
use std::rc::Rc;

use crate::{Artifact, Artifacts};

/// Builder
#[derive(Clone, Deref)]
pub struct Builder(Rc<BuilderData>);

/// Builder
pub struct BuilderData {
    /// Command to execute
    command: String,

    /// Valiables list
    variables: Vec<String>,
    //
    //outputs: Vec<>
}

impl Builder {
    /// Create new builder
    pub fn new<S: Into<String>>(command: S) -> Self {
        let command = command.into();
        let variables = Vec::new();
        Self(Rc::new(BuilderData { command, variables }))
    }

    /// Apply builder to artifacts
    pub fn apply<A: Into<Artifacts>>(
        &self,
        inputs: A, /*, variables: Variables*/
    ) -> Artifacts {
        Artifacts::default()
    }

    pub fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<Self>("Builder")
            .register_fn("Builder", Self::new::<&str>)
            .register_fn("*", Self::apply::<Artifacts>)
            .register_fn("*", Self::apply::<Artifact>);
    }
}

/*
#[derive(Clone, Deref)]
pub struct BuilderInstance(Rc<BuilderInstanceData>);

pub struct BuilderInstanceData {
    builder: Builder,
    //variables: Variables,
    outputs: Artifacts,
}

impl BuilderInstance {
    /// Create builder instance
    pub fn new(builder: Builder) -> Self {
        let outputs = Artifacts::default();

        Self(Rc::new(BuilderInstanceData { builder, outputs }))
    }

    pub fn get_builder(&mut self) -> Builder {
        self.builder.clone()
    }

    pub fn get_outputs(&mut self) -> Artifacts {
        self.outputs.clone()
    }

    pub fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<Self>("BuilderInstance")
            .register_get("builder", Self::get_builder)
            .register_get("outputs", Self::get_outputs);
    }
}
*/
