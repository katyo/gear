use super::{Value, ValueResult};

pub trait Validator {
    fn validate(&self, value: Value) -> ValueResult<Value>;
}
