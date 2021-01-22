use crate::qjs;

pub struct Js;

impl qjs::ObjectDef for Js {
    fn init<'js>(_ctx: qjs::Ctx<'js>, globals: &qjs::Object<'js>) -> qjs::Result<()> {
        let array_class: qjs::Object = globals.get("Array")?;
        let array_proto = array_class.get_prototype()?;

        /*Object.defineProperty(Array.prototype, 'asyncAll', {
          get: function() { return Promise.all(this); }
        })*/
        array_proto.prop(
            "asyncAll",
            qjs::Accessor::from(|ctx, this| call_promise_fn(ctx, this, "all")),
        )?;

        /*Object.defineProperty(Array.prototype, 'asyncAny', {
          get: function() { return Promise.race(this); }
        })*/
        array_proto.prop(
            "asyncAny",
            qjs::Accessor::from(|ctx, this| call_promise_fn(ctx, this, "race")),
        )?;

        Ok(())
    }
}

#[inline]
fn call_promise_fn<'js>(
    ctx: qjs::Ctx<'js>,
    this: qjs::This<qjs::Array<'js>>,
    func: impl AsRef<str>,
) -> qjs::Result<qjs::Object<'js>> {
    let promise_class: qjs::Object = ctx.globals().get("Promise")?;
    let promise_func: qjs::Function = promise_class.get(func.as_ref())?;
    promise_func.call((qjs::This(promise_class), this.0))
}
