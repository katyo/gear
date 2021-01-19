use super::Value;
use crate::{qjs, Map};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
};

const fn default_int_min() -> i64 {
    i64::MIN
}

const fn default_int_max() -> i64 {
    i64::MAX
}

const fn default_float_min() -> f64 {
    f64::MIN
}

const fn default_float_max() -> f64 {
    f64::MAX
}

const fn default_len_min() -> usize {
    usize::MIN
}

const fn default_len_max() -> usize {
    usize::MAX
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, qjs::IntoJs, qjs::FromJs)]
#[serde(tag = "type", rename_all = "lowercase")]
#[quickjs(tag = "type", rename_all = "lowercase")]
pub enum ValueDef {
    Any,
    Bool,
    Int {
        #[serde(default = "default_int_min")]
        #[quickjs(default = "default_int_min")]
        min: i64,
        #[serde(default = "default_int_max")]
        #[quickjs(default = "default_int_max")]
        max: i64,
    },
    Float {
        #[serde(default = "default_float_min")]
        #[quickjs(default = "default_float_min")]
        min: f64,
        #[serde(default = "default_float_max")]
        #[quickjs(default = "default_float_max")]
        max: f64,
    },
    String {
        #[serde(default = "default_len_min")]
        #[quickjs(default = "default_len_min")]
        min: usize,
        #[serde(default = "default_len_max")]
        #[quickjs(default = "default_len_max")]
        max: usize,
    },
    Option {
        value: Box<ValueDef>,
    },
    Either {
        options: Vec<ValueDef>,
    },
    Enum {
        value: Box<ValueDef>,
        options: Vec<Value>,
    },
    Tuple {
        values: Vec<ValueDef>,
    },
    Record {
        fields: Map<String, ValueDef>,
    },
    List {
        value: Box<ValueDef>,
        #[serde(default = "default_len_min")]
        #[quickjs(default = "default_len_min")]
        min: usize,
        #[serde(default = "default_len_max")]
        #[quickjs(default = "default_len_max")]
        max: usize,
    },
    Dict {
        value: Box<ValueDef>,
        #[serde(default = "default_len_min")]
        #[quickjs(default = "default_len_min")]
        min: usize,
        #[serde(default = "default_len_max")]
        #[quickjs(default = "default_len_max")]
        max: usize,
    },
}

impl ValueDef {
    pub fn default_value(&self) -> Value {
        Value::default_for(self)
    }
}

impl<'js> qjs::IntoJs<'js> for &ValueDef {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.clone().into_js(ctx)
    }
}

impl Default for ValueDef {
    fn default() -> Self {
        ValueDef::Any
    }
}

impl Display for ValueDef {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ValueDef::Any => "any".fmt(f),
            ValueDef::Bool => "bool".fmt(f),
            ValueDef::Int { min, max } => {
                "int [".fmt(f)?;
                min.fmt(f)?;
                "..".fmt(f)?;
                max.fmt(f)?;
                ']'.fmt(f)
            }
            ValueDef::Float { min, max } => {
                "float [".fmt(f)?;
                min.fmt(f)?;
                "..".fmt(f)?;
                max.fmt(f)?;
                ']'.fmt(f)
            }
            ValueDef::String { min, max } => {
                "string [".fmt(f)?;
                min.fmt(f)?;
                "..".fmt(f)?;
                max.fmt(f)?;
                ']'.fmt(f)
            }
            ValueDef::Option { value } => {
                "option<".fmt(f)?;
                value.fmt(f)?;
                '>'.fmt(f)
            }
            ValueDef::Either { options } => {
                "either<".fmt(f)?;
                let mut iter = options.iter();
                if let Some(value) = iter.next() {
                    value.fmt(f)?;
                    for value in iter {
                        ", ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                '>'.fmt(f)
            }
            ValueDef::Enum { value, options } => {
                "enum<".fmt(f)?;
                value.fmt(f)?;
                "> [".fmt(f)?;
                let mut iter = options.iter();
                if let Some(value) = iter.next() {
                    value.fmt(f)?;
                    for value in iter {
                        ", ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                ']'.fmt(f)
            }
            ValueDef::Tuple { values } => {
                "tuple<".fmt(f)?;
                let mut iter = values.iter();
                if let Some(value) = iter.next() {
                    value.fmt(f)?;
                    for value in iter {
                        ", ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                '>'.fmt(f)
            }
            ValueDef::Record { fields } => {
                "rec<".fmt(f)?;
                let mut iter = fields.iter();
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
                '>'.fmt(f)
            }
            ValueDef::List { value, min, max } => {
                "list<".fmt(f)?;
                value.fmt(f)?;
                "> [".fmt(f)?;
                min.fmt(f)?;
                "..".fmt(f)?;
                max.fmt(f)?;
                ']'.fmt(f)
            }
            ValueDef::Dict { value, min, max } => {
                "dict<".fmt(f)?;
                value.fmt(f)?;
                "> [".fmt(f)?;
                min.fmt(f)?;
                "..".fmt(f)?;
                max.fmt(f)?;
                ']'.fmt(f)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    pub name: String,
    pub description: String,
    #[serde(flatten)]
    pub definition: ValueDef,
    pub default: Value,
}

impl VariableDef {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        definition: Option<ValueDef>,
        default: Option<Value>,
    ) -> Self {
        let definition = definition.unwrap_or_default();
        let default = default.unwrap_or_else(|| Value::default_for(&definition));
        Self {
            name: name.into(),
            description: description.into(),
            definition,
            default,
        }
    }
}

impl Display for VariableDef {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.name.fmt(f)?;
        ": ".fmt(f)?;
        self.definition.fmt(f)
    }
}

impl Borrow<str> for VariableDef {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl Borrow<String> for VariableDef {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl PartialEq<VariableDef> for VariableDef {
    fn eq(&self, other: &VariableDef) -> bool {
        self.name == other.name
    }
}

impl Eq for VariableDef {}

impl Hash for VariableDef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
