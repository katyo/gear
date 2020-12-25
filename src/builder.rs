use crate::{qjs, AnyKind, Artifact, Input, Mut, Output, Ref, Result, Set, WeakArtifact, WeakSet};
use derive_deref::Deref;
use either::Either;
use std::{future::Future, iter::once, pin::Pin};

/// The builder interface
pub trait BuilderApi {
    //// Get the list of values
    //fn values(&self) -> Vec<Ref<Value>>;

    /// Get the list of inputs
    fn inputs(&self) -> Vec<Artifact<Input>>;

    /// Get the list of outputs
    fn outputs(&self) -> Vec<Artifact<Output>>;

    /// Run builder
    fn build(&self) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}

#[derive(Clone, Deref)]
pub struct Builder(Ref<dyn BuilderApi>);

pub struct NoInternal {
    inputs: Mut<Set<Artifact<Input>>>,
    outputs: WeakSet<WeakArtifact<Output>>,
}

impl Drop for NoInternal {
    fn drop(&mut self) {
        log::debug!("NoBuilder::drop");
    }
}

#[derive(Clone, Deref)]
#[repr(transparent)]
pub struct NoBuilder(Ref<NoInternal>);

impl NoBuilder {
    fn to_dyn(&self) -> Builder {
        Builder(Ref::new(self.0.clone()))
    }

    fn new_raw(inputs: Set<Artifact<Input>>, outputs: WeakSet<WeakArtifact<Output>>) -> Self {
        let inputs = Mut::new(inputs);
        let this = Self(Ref::new(NoInternal { inputs, outputs }));
        log::debug!("NoBuilder::new");
        {
            let builder = this.to_dyn();
            for output in &this.0.outputs {
                output.set_builder(builder.clone());
            }
        }
        this
    }
}

impl BuilderApi for Ref<NoInternal> {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.outputs.iter().collect()
    }

    fn build(&self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(qjs::HasRefs)]
pub struct JsInternal {
    inputs: Mut<Set<Artifact<Input>>>,
    outputs: WeakSet<WeakArtifact<Output>>,
    #[quickjs(has_refs)]
    build: qjs::Persistent<qjs::Function<'static>>,
    context: qjs::Context,
}

impl Drop for JsInternal {
    fn drop(&mut self) {
        log::debug!("JsBuilder::drop");
    }
}

#[derive(Clone, Deref, qjs::HasRefs)]
#[repr(transparent)]
pub struct JsBuilder(#[quickjs(has_refs)] Ref<JsInternal>);

impl JsBuilder {
    fn to_dyn(&self) -> Builder {
        Builder(Ref::new(self.0.clone()))
    }

    fn new_raw(
        inputs: Set<Artifact<Input>>,
        outputs: WeakSet<WeakArtifact<Output>>,
        build: qjs::Persistent<qjs::Function<'static>>,
        context: qjs::Context,
    ) -> Self {
        let inputs = Mut::new(inputs);
        let this = Self(Ref::new(JsInternal {
            inputs,
            outputs,
            build,
            context,
        }));
        log::debug!("JsBuilder::new");
        {
            let builder = this.to_dyn();
            for output in &this.0.outputs {
                output.set_builder(builder.clone());
            }
        }
        this
    }
}

impl BuilderApi for Ref<JsInternal> {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
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
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs()
        }

        #[quickjs(get)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.outputs()
        }
    }

    #[quickjs(rename = "NoBuilder")]
    impl NoBuilder {
        pub fn new(
            outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
            inputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>>,
        ) -> Self {
            let inputs = inputs
                .0
                .map(|inputs| {
                    inputs.either(
                        |inputs| inputs.into_iter().map(|input| input.0.clone()).collect(),
                        |input| once(input.0.clone()).collect(),
                    )
                })
                .unwrap_or_default();
            let outputs = outputs
                .0
                .map(|outputs| {
                    outputs.either(
                        |outputs| outputs.into_iter().map(|output| output.0.clone()).collect(),
                        |output| once(output.0.clone()).collect(),
                    )
                })
                .unwrap_or_default();
            Self::new_raw(inputs, outputs)
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs.read().iter().cloned().collect()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(
            &self,
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        ) {
            *self.0.inputs.write() = inputs.either(
                |inputs| inputs.into_iter().map(|input| input.0.clone()).collect(),
                |input| once(input.0.clone()).collect(),
            );
        }

        #[quickjs(get)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.outputs.iter().collect()
        }
    }

    #[quickjs(rename = "FnBuilder", has_refs)]
    impl JsBuilder {
        pub fn new<'js>(
            ctx: qjs::Ctx<'js>,
            build: qjs::Persistent<qjs::Function<'static>>,
            outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
            inputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>>,
        ) -> Self {
            let context = qjs::Context::from_ctx(ctx).unwrap();
            let inputs = inputs
                .0
                .map(|inputs| {
                    inputs.either(
                        |inputs| inputs.into_iter().map(|input| input.0.clone()).collect(),
                        |input| once(input.0.clone()).collect(),
                    )
                })
                .unwrap_or_default();
            let outputs = outputs
                .0
                .map(|outputs| {
                    outputs.either(
                        |outputs| outputs.into_iter().map(|output| output.0.clone()).collect(),
                        |output| once(output.0.clone()).collect(),
                    )
                })
                .unwrap_or_default();
            Self::new_raw(inputs, outputs, build, context)
        }

        #[quickjs(get)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs.read().iter().cloned().collect()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(
            &self,
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        ) {
            *self.0.inputs.write() = inputs.either(
                |inputs| inputs.into_iter().map(|input| input.0.clone()).collect(),
                |input| once(input.0.clone()).collect(),
            );
        }

        #[quickjs(get)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.outputs.iter().collect()
        }
    }
}
