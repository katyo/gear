use crate::{qjs, Artifact, Mut, Ref, Result, Set, WeakArtifact, WeakSet};
use derive_deref::Deref;
use either::Either;
use std::{future::Future, iter::once, pin::Pin};

/// The builder interface
pub trait BuilderApi {
    //// Get the list of values
    //fn values(&self) -> Vec<Ref<Value>>;

    /// Get the list of inputs
    fn inputs(&self) -> Vec<Artifact>;

    /// Get the list of outputs
    fn outputs(&self) -> Vec<Artifact>;

    /// Run builder
    fn build(&self) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}

#[derive(Clone, Deref)]
pub struct Builder(Ref<dyn BuilderApi>);

pub struct Internal {
    inputs: Mut<Set<Artifact>>,
    outputs: WeakSet<WeakArtifact>,
    build: qjs::Persistent<qjs::Function<'static>>,
    context: qjs::Context,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("JsBuilder::drop");
    }
}

#[derive(Clone, Deref)]
#[repr(transparent)]
pub struct JsBuilder(Ref<Internal>);

impl JsBuilder {
    fn to_dyn(&self) -> Builder {
        Builder(Ref::new(self.0.clone()))
    }
}

impl BuilderApi for Ref<Internal> {
    fn inputs(&self) -> Vec<Artifact> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact> {
        self.outputs.iter().collect()
    }

    fn build(&self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        let build = self.build.clone();
        let context = self.context.clone();
        let this = JsBuilder(self.clone());
        Box::pin(async move {
            let promise: qjs::Promise<()> =
                context.with(|ctx| build.restore(ctx)?.call((qjs::This(this),)))?;
            Ok(promise.await?)
        })
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(rename = "AnyBuilder")]
    impl Builder {
        pub fn new() -> Self {
            unimplemented!();
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact> {
            self.0.inputs()
        }
    }

    #[quickjs(rename = "Builder")]
    impl JsBuilder {
        pub fn new<'js>(
            ctx: qjs::Ctx<'js>,
            build: qjs::Persistent<qjs::Function<'static>>,
            outputs: qjs::Opt<Either<Vec<&Artifact>, &Artifact>>,
            inputs: qjs::Opt<Either<Vec<&Artifact>, &Artifact>>,
        ) -> Self {
            let context = qjs::Context::from_ctx(ctx).unwrap();
            let inputs = Mut::new(
                inputs
                    .0
                    .map(|inputs| {
                        inputs.either(
                            |inputs| inputs.into_iter().cloned().collect(),
                            |input| once(input.clone()).collect(),
                        )
                    })
                    .unwrap_or_default(),
            );
            let outputs = outputs
                .0
                .map(|outputs| {
                    outputs.either(
                        |outputs| outputs.into_iter().cloned().collect(),
                        |output| once(output.clone()).collect(),
                    )
                })
                .unwrap_or_default();
            log::debug!("JsBuilder::new");
            let builder = Self(Ref::new(Internal {
                inputs,
                outputs,
                build,
                context,
            }));
            {
                let dyn_builder = builder.to_dyn();
                for output in &builder.0.outputs {
                    output.set_builder(dyn_builder.clone());
                }
            }
            builder
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact> {
            self.0.inputs.read().iter().cloned().collect()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(&self, inputs: Vec<&Artifact>) {
            *self.0.inputs.write() = inputs.into_iter().cloned().collect();
        }

        #[quickjs(get)]
        pub fn outputs(&self) -> Vec<Artifact> {
            self.0.outputs.iter().collect()
        }
    }
}
