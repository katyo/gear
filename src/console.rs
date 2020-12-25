use crate::qjs;

#[qjs::bind(object, public)]
#[quickjs(rename = "console")]
mod js {
    use rquickjs::{Coerced, Rest};

    fn join_args(args: Rest<Coerced<String>>) -> String {
        args.0
            .into_iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn log(args: Rest<Coerced<String>>) {
        log::info!(target: "gear::js", "{}", join_args(args));
    }

    pub fn error(args: Rest<Coerced<String>>) {
        log::error!(target: "gear::js", "{}", join_args(args));
    }

    pub fn warn(args: Rest<Coerced<String>>) {
        log::warn!(target: "gear::js", "{}", join_args(args));
    }

    pub fn info(args: Rest<Coerced<String>>) {
        log::info!(target: "gear::js", "{}", join_args(args));
    }

    pub fn debug(args: Rest<Coerced<String>>) {
        log::debug!(target: "gear::js", "{}", join_args(args));
    }

    pub fn trace(args: Rest<Coerced<String>>) {
        log::trace!(target: "gear::js", "{}", join_args(args));
    }
}
