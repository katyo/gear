use crate::{
    qjs,
    system::{create_dir_all, Path},
    Artifact, BoxedFuture, Diagnostics, Input, Mut, Output, ParallelSend, ParallelSync, Ref,
    Result, Set, Time, WeakArtifact, WeakSet,
};
use derive_deref::Deref;
use either::Either;
use futures::future::FutureExt;
use serde::Serialize;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    iter::once,
};

/// The unique identifier of rule
pub type RuleId = u64;

/// The rule processing state
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u32)]
pub enum RuleState {
    Processed,
    Scheduled,
    Processing,
}

impl Default for RuleState {
    fn default() -> Self {
        Self::Processed
    }
}

impl Display for RuleState {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match self {
            RuleState::Processed => "processed",
            RuleState::Scheduled => "scheduled",
            RuleState::Processing => "processing",
        }
        .fmt(fmt)
    }
}

/// The builder interface
pub trait RuleApi: ParallelSend + ParallelSync {
    /// Get the list of inputs
    fn inputs(&self) -> Vec<Artifact<Input>>;

    /// Get the list of outputs
    fn outputs(&self) -> Vec<Artifact<Output>>;

    /// Run rule
    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>>;
}

#[derive(Clone)]
pub struct Rule(Ref<Internal>);

struct Internal {
    id: RuleId,
    state: Mut<RuleState>,
    diagnostics: Mut<Diagnostics>,
    api: Ref<dyn RuleApi>,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Rule {}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "Rule #".fmt(f)?;
        self.0.id.fmt(f)
    }
}

impl Rule {
    pub fn from_api(api: Ref<dyn RuleApi>) -> Self {
        let mut hasher = fxhash::FxHasher::default();
        for output in api.outputs() {
            output.hash(&mut hasher);
        }
        let id = hasher.finish();
        let state = Mut::new(RuleState::default());
        let diagnostics = Mut::new(Diagnostics::default());

        Self(Ref::new(Internal {
            id,
            api,
            state,
            diagnostics,
        }))
    }

    pub fn id(&self) -> RuleId {
        self.0.id
    }

    pub fn state(&self) -> RuleState {
        *self.0.state.read()
    }

    pub fn ready_inputs(&self) -> bool {
        let inputs = self.0.api.inputs();
        inputs.is_empty() || !inputs.into_iter().any(|input| input.outdated())
    }

    pub fn schedule(&self) {
        *self.0.state.write() = RuleState::Scheduled;
    }

    pub async fn process(&self) -> Result<()> {
        {
            *self.0.state.write() = RuleState::Processing;
        }
        for output in self.0.api.outputs() {
            if let Some(dir) = Path::new(output.name()).parent() {
                if !dir.is_dir().await {
                    create_dir_all(dir).await?;
                }
            }
        }
        let diagnostics = self.0.api.clone().invoke().await?;
        let is_failed = diagnostics.is_failed();
        {
            *self.0.diagnostics.write() = diagnostics;
        }
        if is_failed {
            Err(format!("Failed processing rule"))?;
        }
        let time = Time::now();
        for output in self.0.api.outputs() {
            output.set_time(time);
        }
        {
            *self.0.state.write() = RuleState::Processed;
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

impl Display for NoRule {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "NoRule".fmt(f)
    }
}

impl NoRule {
    fn to_dyn(&self) -> Rule {
        Rule::from_api(self.0.clone())
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

impl RuleApi for NoInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.outputs.iter().collect()
    }

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        async { Ok(Diagnostics::default()) }.boxed_local()
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

#[cfg(feature = "parallel")]
unsafe impl Send for JsInternal {}
#[cfg(feature = "parallel")]
unsafe impl Sync for JsInternal {}

impl Drop for JsInternal {
    fn drop(&mut self) {
        log::debug!("JsRule::drop");
    }
}

#[derive(Clone, Deref, qjs::HasRefs)]
#[repr(transparent)]
pub struct JsRule(#[quickjs(has_refs)] Ref<JsInternal>);

impl Display for JsRule {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "JsRule".fmt(f)
    }
}

impl JsRule {
    fn to_dyn(&self) -> Rule {
        Rule::from_api(self.0.clone())
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

impl RuleApi for JsInternal {
    fn inputs(&self) -> Vec<Artifact<Input>> {
        self.inputs.read().iter().cloned().collect()
    }

    fn outputs(&self) -> Vec<Artifact<Output>> {
        self.outputs.iter().collect()
    }

    fn invoke(self: Ref<Self>) -> BoxedFuture<Result<Diagnostics>> {
        let function = self.function.clone();
        let context = self.context.clone();
        let this = JsRule(self);
        async move {
            let promise: qjs::Promise<_> =
                context.with(|ctx| function.restore(ctx)?.call((qjs::This(this),)))?;
            Ok(promise.await?)
        }
        .boxed_local()
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

        #[quickjs(get, enumerable)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.api.inputs()
        }

        #[quickjs(get, enumerable)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.api.outputs()
        }

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_js1<'js>(
        inputs: Either<Set<Artifact<Input>>, Artifact<Input>>,
        outputs: Either<Set<Artifact<Output>>, Artifact<Output>>,
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
        function: qjs::Persistent<qjs::Function<'static>>,
        outputs: Either<Set<Artifact<Output>>, Artifact<Output>>,
        inputs: Either<Set<Artifact<Input>>, Artifact<Input>>,
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
        outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
        inputs: qjs::Opt<Either<Set<Artifact<Input>>, Artifact<Input>>>,
        ctx: qjs::Ctx<'js>,
    ) -> JsRule {
        JsRule::new_(function, outputs, inputs, ctx)
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_no1<'js>(
        inputs: Either<Set<Artifact<Input>>, Artifact<Input>>,
        outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
    ) -> NoRule {
        NoRule::new_(outputs, qjs::Opt(Some(inputs)))
    }

    #[quickjs(rename = "Rule")]
    pub fn rule_no2<'js>(
        outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
        inputs: qjs::Opt<Either<Set<Artifact<Input>>, Artifact<Input>>>,
    ) -> NoRule {
        NoRule::new_(outputs, inputs)
    }

    #[quickjs(rename = "NoRule")]
    impl NoRule {
        #[quickjs(rename = "new")]
        pub fn new(
            inputs: Either<Set<Artifact<Input>>, Artifact<Input>>,
            outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
        ) -> Self {
            Self::new_(outputs, qjs::Opt(Some(inputs)))
        }

        #[quickjs(rename = "new")]
        pub fn new_(
            outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
            inputs: qjs::Opt<Either<Set<Artifact<Input>>, Artifact<Input>>>,
        ) -> Self {
            let inputs = inputs
                .0
                .map(|inputs| inputs.either(|inputs| inputs, |input| once(input).collect()))
                .unwrap_or_default();
            let outputs = outputs
                .0
                .map(|outputs| {
                    outputs.either(
                        |outputs| outputs.into_iter().collect(),
                        |output| once(output).collect(),
                    )
                })
                .unwrap_or_default();
            Self::new_raw(inputs, outputs)
        }

        #[quickjs(get, enumerable)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs.read().iter().cloned().collect()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(&self, inputs: Either<Set<Artifact<Input>>, Artifact<Input>>) {
            *self.0.inputs.write() = inputs.either(|inputs| inputs, |input| once(input).collect());
        }

        #[quickjs(get, enumerable)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.outputs.iter().collect()
        }

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }

    #[quickjs(rename = "FnRule", has_refs)]
    impl JsRule {
        pub fn new<'js>(
            inputs: Either<Set<Artifact<Input>>, Artifact<Input>>,
            outputs: Either<Set<Artifact<Output>>, Artifact<Output>>,
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
            outputs: qjs::Opt<Either<Set<Artifact<Output>>, Artifact<Output>>>,
            inputs: qjs::Opt<Either<Set<Artifact<Input>>, Artifact<Input>>>,
            ctx: qjs::Ctx<'js>,
        ) -> Self {
            let context = qjs::Context::from_ctx(ctx).unwrap();
            let inputs = inputs
                .0
                .map(|inputs| inputs.either(|inputs| inputs, |input| once(input).collect()))
                .unwrap_or_default();
            let outputs = outputs
                .0
                .map(|outputs| {
                    outputs.either(
                        |outputs| outputs.into_iter().collect(),
                        |output| once(output).collect(),
                    )
                })
                .unwrap_or_default();
            Self::new_raw(inputs, outputs, function, context)
        }

        #[quickjs(get, enumerable)]
        pub fn inputs(&self) -> Vec<Artifact<Input>> {
            self.0.inputs.read().iter().cloned().collect()
        }

        #[quickjs(rename = "inputs", set)]
        pub fn set_inputs(&self, inputs: Either<Set<Artifact<Input>>, Artifact<Input>>) {
            *self.0.inputs.write() = inputs.either(|inputs| inputs, |input| once(input).collect());
        }

        #[quickjs(get, enumerable)]
        pub fn outputs(&self) -> Vec<Artifact<Output>> {
            self.0.outputs.iter().collect()
        }

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }
}
