use crate::{Error, Map, Result};
use either::{Either, Left, Right};
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdReult,
};

#[derive(Debug, Clone)]
pub enum ValueError {
    Mismatch {
        type_: Type,
        given: Value,
    },
    TooSmall {
        type_: Type,
        min: f64,
        given: f64,
    },
    TooBig {
        type_: Type,
        max: f64,
        given: f64,
    },
    NotEnough {
        type_: Type,
        min: usize,
        given: usize,
    },
    Exceeded {
        type_: Type,
        max: usize,
        given: usize,
    },
    Errors(Vec<ValueError>),
}

/*impl ValueError {
    pub fn mismatch_type(type_: &Type, given: &Type) -> Self {

    }
}*/

impl Error for ValueError {}

impl Display for ValueError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        use ValueError::*;
        match self {
            Mismatch { expected, given } => {
                "The ".fmt(f)?;
                expected.fmt(f)?;
                " value expected but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            TooSmall { type_, min, given } => {
                "The ".fmt(f)?;
                type_.fmt(f)?;
                " value should not be less than ".fmt(f)?;
                min.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            TooBig { type_, max, given } => {
                "The ".fmt(f)?;
                type_.fmt(f)?;
                " value should not be greater than ".fmt(f)?;
                max.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            NotEnough { type_, min, given } => {
                "The ".fmt(f)?;
                type_.fmt(f)?;
                " value should not be shorter than ".fmt(f)?;
                min.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            Exceeded { type_, max, given } => {
                "The ".fmt(f)?;
                type_.fmt(f)?;
                " value should not be longer than ".fmt(f)?;
                max.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    Unknown,
    Bool,
    Int {
        min: i32,
        max: i32,
    },
    Float {
        min: f64,
        max: f64,
    },
    String {
        min: usize,
        max: usize,
    },
    Option {
        item: Type,
    },
    Either {
        item: Vec<Type>,
    },
    Tuple {
        items: Vec<Type>,
    },
    List {
        item: Box<Type>,
        min: usize,
        max: usize,
    },
    Dict {
        item: Box<Type>,
        min: usize,
        max: usize,
    },
    Rec {
        items: Map<String, Type>,
    },
    Enum {
        item: Box<Type>,
        opts: Vec<Value>,
    },
}

impl Type {
    pub fn mismatch(&self, value: &Value) -> ValueError {
        ValueError::Mismatch {
            type_: self.clone,
            given: value.clone(),
        }
    }

    /*pub fn sameas(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Unknown, Type::Unknown) | (Type::Bool, Type::Bool) | (Type::Int { .. }, Type::Int { .. }) | (Type::Float { .. }, Type::Float { .. }) | (Type::String {..}, Type::String { .. }) => true,
            (Type::Option { item: this }, Type::Option { item: other }) if this.sameas(other) => true,

        }
    }*/
}

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Dict(Map<String, Value>),
}

impl Value {
    /*pub fn typeof(&self) -> Type {
        match self {
            Value::None => Type::Unknown,
            Value::Bool(_) => Type::Bool,
            Value::Int(val) => Type::Int { min: val, max: val },
            Value::Float(val) => Type::Float { min: val, max: val },
            Value::String(val) => Type::String { min: val.len(), max: val.len() },
            Value::List(val) => {
                let types = val.iter().map(|val| val.typeof()).collect::<Vec<_>>();
                types.iter().fold()
                Type::List
            }
        }
    }*/

    pub fn check(&self, type_: &Type) -> StdResult<(), ValueError> {
        use ValueError::*;
        match type_ {
            Type::Bool => match self {
                Value::Bool(_) => Ok(()),
                given => Err(type_.mismatch(given)),
            },
            Type::Int { min, max } => match self {
                Value::Int(given) => {
                    if given < min {
                        Err(TooSmall { min, given })
                    } else if given > max {
                        Err(TooBig { max, given })
                    } else {
                        Ok(())
                    }
                }
                Value::Float(given) if given.fract() == 0.0 => {
                    let given = given as i32;
                    if given < min {
                        Err(TooSmall {
                            min: min as _,
                            given: given as _,
                        })
                    } else if given > max {
                        Err(TooBig {
                            max: max as _,
                            given: given as _,
                        })
                    } else {
                        Ok(())
                    }
                }
                given => Err(type_.mismatch(given)),
            },
            Type::Float { min, max } => match self {
                Value::Float(val) => {
                    if val < min {
                        Err(TooSmall { min, given })
                    } else if val > max {
                        Err(TooBig { max, given })
                    } else {
                        Ok(())
                    }
                }
                Value::Int(val) => {
                    let val = val as f64;
                    if val < min {
                        Err(TooSmall { min, given })
                    } else if val > max {
                        Err(TooSmall { max, given })
                    } else {
                        Ok(())
                    }
                }
                val => Err(type_.mismatch(given)),
            },
            Type::String { min, max } => match self {
                Value::String(val) => {
                    let len = val.len();
                    if len < min {
                        Err(NotEnough {
                            type_: type_.clone(),
                            min,
                            len,
                        })
                    } else if len > max {
                        Err(Exceeded { max, len })
                    } else {
                        Ok(())
                    }
                }
            },
            Type::Option { item } => match self {
                Value::None => Ok(()),
                value => value.check(item),
            },
            Type::Either { item } => item
                .iter()
                .map(|type_| value.check(type_))
                .collect::<Result<_, Vec<_>>>()
                .map_err(ValueError::Errors),

            Type::Tuple { items } => match self {
                Value::List(values) => {
                    let items_len = items.len();
                    let values_len = value.len();
                    if values_len < items_len {
                        Err(NotEnough {
                            type_: type_.clone(),
                            min: values_len,
                        })
                    } else if values_len > items_len {
                        Err(format!(
                            "Too much values in list. A {} expected but {} given",
                            items_len, value_len
                        ))
                    } else {
                        for index in 0..items_len {
                            if let Err(error) = values[index].check(items[index]) {
                                return Err(error);
                            }
                        }
                        Ok(())
                    }
                }
                value => Err(format!("A list of values expected but {} given", value)),
            },
        }
    }
}

pub struct Variable {
    type_: Type,
    value: Value,
}
