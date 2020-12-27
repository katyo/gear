use crate::{
    qjs, AnyKind, Artifact, Input, Mut, Output, Ref, Result, Set, Time, WeakArtifact, WeakSet,
};
use derive_deref::Deref;
use either::Either;
use std::{
    future::Future,
    hash::{Hash, Hasher},
    iter::once,
    pin::Pin,
};

/// The builder interface
pub trait RuleApi {
    //// Get the list of values
    //fn values(&self) -> Vec<Ref<Value>>;

    /// Get the list of inputs
    fn inputs(&self) -> Vec<Artifact<Input>>;

    /// Get the list of outputs
    fn outputs(&self) -> Vec<Artifact<Output>>;

    /// Run rule
    fn invoke(&self) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}

#[derive(Clone, Deref)]
pub struct Rule(Ref<dyn RuleApi>);

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        Ref::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Rule {}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Ref::as_ptr(&self.0).hash(state);
    }
}

impl Rule {
    pub fn ready_inputs(&self) -> bool {
        let inputs = self.0.inputs();
        inputs.is_empty() || !inputs.into_iter().any(|input| input.outdated())
    }

    pub async fn process(self) -> Result<()> {
        self.0.invoke().await?;
        let time = Time::now();
        for output in self.0.outputs() {
            output.set_time(time);
        }
        Ok(())
    }
}

pub struct NoInternal {
    inputs: Mut<Set<Artifact<Input>>>,
    outputs: WeakSet<WeakArtifact<Output>>,
}

impl Drop for NoInternal {
    fn drop(&mut self) {
        log::debug!("NoRule::drop");
    }
}

#[derive(Clone, Deref)]
#[repr(transparent)]
pub struct NoRule(Ref<NoInternal>);

impl NoRule {
    fn to_dyn(&self) -> Rule {
        Rule(Ref::new(self.0.clone()))
    }

    pub fn new_raw(inputs: Set<Artifact<Input>>, outputs: WeakSet<WeakArtifact<Output>>) -> Self {
        let inputs = Mut::new(inputs);
        let this = Self(Ref::new(NoInternal { inputs, outputs }));
        log::debug!("NoRule::new");
        {
            let rule = this.to_dyn();
            for output in &this.0.outputs {
                output.set_rule(rule.clone());
            }
        }
        this
    }
}

impl RuleApi for Ref<NoInternal> {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.outputs.iter().collect()
    }

    fn invoke(&self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(qjs::HasRefs)]
pub struct JsInternal {
    inputs: Mut<Set<Artifact<Input>>>,
    outputs: WeakSet<WeakArtifact<Output>>,
    #[quickjs(has_refs)]
    function: qjs::Persistent<qjs::Function<'static>>,
    context: qjs::Context,
}

impl Drop for JsInternal {
    fn drop(&mut self) {
        log::debug!("JsRule::drop");
    }
}

#[derive(Clone, Deref, qjs::HasRefs)]
#[repr(transparent)]
pub struct JsRule(#[quickjs(has_refs)] Ref<JsInternal>);

impl JsRule {
    fn to_dyn(&self) -> Rule {
        Rule(Ref::new(self.0.clone()))
    }

    pub fn new_raw(
        inputs: Set<Artifact<Input>>,
        outputs: WeakSet<WeakArtifact<Output>>,
        function: qjs::Persistent<qjs::Function<'static>>,
        context: qjs::Context,
    ) -> Self {
        let inputs = Mut::new(inputs);
        let this = Self(Ref::new(JsInternal {
            inputs,
            outputs,
            function,
            context,
        }));
        log::debug!("JsRule::new");
        {
            let rule = this.to_dyn();
            for output in &this.0.outputs {
                output.set_rule(rule.clone());
            }
        }
        this
    }
}

impl RuleApi for Ref<JsInternal> {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.outputs.iter().collect()
    }

    fn invoke(&self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        let function = self.function.clone();
        let context = self.context.clone();
        let this = JsRule(self.clone());
        Box::pin(async move {
            let promise: qjs::Promise<()> =
                context.with(|ctx| function.restore(ctx)?.call((qjs::This(this),)))?;
            Ok(promise.await?)
        })
    }
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    #[quickjs(rename = "AnyRule")]
    impl Rule {
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

    #[quickjs(rename = "Rule")]
    pub fn rule_js1<'js>(
        inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        outputs: Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>,
        function: qjs::Persistent<qjs::Function<'static>>,
        ctx: qjs::Ctx<'js>,
    ) -> JsRule {
        JsRule::new_(
            function,
            qjs::Opt(Some(outputs)),
            qjs::Opt(Some(inputs)),
            ctx,
        )
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_js2<'js>(
        outputs: Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>,
        inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        function: qjs::Persistent<qjs::Function<'static>>,
        ctx: qjs::Ctx<'js>,
    ) -> JsRule {
        JsRule::new_(
            function,
            qjs::Opt(Some(outputs)),
            qjs::Opt(Some(inputs)),
            ctx,
        )
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_js3<'js>(
        function: qjs::Persistent<qjs::Function<'static>>,
        outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
        inputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>>,
        ctx: qjs::Ctx<'js>,
    ) -> JsRule {
        JsRule::new_(function, outputs, inputs, ctx)
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_no1<'js>(
        inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
        outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
    ) -> NoRule {
        NoRule::new_(outputs, qjs::Opt(Some(inputs)))
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_no2<'js>(
        outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
        inputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>>,
    ) -> NoRule {
        NoRule::new_(outputs, inputs)
    }

    #[quickjs(rename = "NoRule")]
    impl NoRule {
        #[quickjs(rename = "new")]
        pub fn new(
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
            outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
        ) -> Self {
            Self::new_(outputs, qjs::Opt(Some(inputs)))
        }

        #[quickjs(rename = "new")]
        pub fn new_(
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

    #[quickjs(rename = "FnRule", has_refs)]
    impl JsRule {
        pub fn new<'js>(
            inputs: Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>,
            outputs: Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>,
            function: qjs::Persistent<qjs::Function<'static>>,
            ctx: qjs::Ctx<'js>,
        ) -> Self {
            Self::new_(
                function,
                qjs::Opt(Some(outputs)),
                qjs::Opt(Some(inputs)),
                ctx,
            )
        }

        #[quickjs(rename = "new")]
        pub fn new_<'js>(
            function: qjs::Persistent<qjs::Function<'static>>,
            outputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Output>>>, AnyKind<&Artifact<Output>>>>,
            inputs: qjs::Opt<Either<Vec<AnyKind<&Artifact<Input>>>, AnyKind<&Artifact<Input>>>>,
            ctx: qjs::Ctx<'js>,
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
            Self::new_raw(inputs, outputs, function, context)
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
