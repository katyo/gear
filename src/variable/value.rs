use super::ValueDef;
use crate::{qjs, Map};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, qjs::IntoJs, qjs::FromJs)]
#[serde(untagged, rename_all = "lowercase")]
#[quickjs(untagged, rename_all = "lowercase")]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Dict(Map<String, Value>),
}

impl<'js> qjs::IntoJs<'js> for &Value {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.clone().into_js(ctx)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

impl Value {
    pub fn default_for(def: &ValueDef) -> Self {
        match def {
            ValueDef::Any {} | ValueDef::Option { .. } => Value::None,
            ValueDef::Bool {} => Value::Bool(false),
            ValueDef::Int { min, .. } => Value::Int((*min).max(0)),
            ValueDef::Float { min, .. } => Value::Float((*min).max(0.0)),
            ValueDef::String { min, .. } => {
                Value::String((0..*min).map(|n| ((n % 10) as u8 + b'0') as char).collect())
            }
            ValueDef::Either { options } => options
                .iter()
                .next()
                .map(Value::default_for)
                .unwrap_or(Value::None),
            ValueDef::Enum { options, .. } => options.iter().next().cloned().unwrap_or(Value::None),
            ValueDef::Tuple { values } => {
                Value::List(values.iter().map(Value::default_for).collect())
            }
            ValueDef::Record { fields } => Value::Dict(
                fields
                    .iter()
                    .map(|(field, value)| (field.clone(), Value::default_for(value)))
                    .collect(),
            ),
            ValueDef::List { value, min, .. } => {
                Value::List((0..*min).map(|_| Value::default_for(value)).collect())
            }
            ValueDef::Dict { value, min, .. } => Value::Dict(
                (0..*min)
                    .map(|n| (n.to_string(), Value::default_for(value)))
                    .collect(),
            ),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Value::None => "none".fmt(f),
            Value::Bool(value) => if *value { "true" } else { "false" }.fmt(f),
            Value::Int(value) => value.fmt(f),
            Value::Float(value) => value.fmt(f),
            Value::String(value) => fmt::Debug::fmt(value, f),
            Value::List(values) => {
                '['.fmt(f)?;
                let mut iter = values.iter();
                if let Some(value) = iter.next() {
                    value.fmt(f)?;
                    for value in iter {
                        ", ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                ']'.fmt(f)
            }
            Value::Dict(values) => {
                '{'.fmt(f)?;
                let mut iter = values.iter();
                if let Some((field, value)) = iter.next() {
                    field.fmt(f)?;
                    ": ".fmt(f)?;
                    value.fmt(f)?;
                    for (field, value) in iter {
                        ", ".fmt(f)?;
                        field.fmt(f)?;
                        ": ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                '}'.fmt(f)
            }
        }
    }
}
