use super::{Value, ValueDef};
use serde::{Deserialize, Serialize};
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
};

pub type ValueResult<T> = StdResult<T, ValueError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueError {
    Mismatch {
        expected: ValueDef,
        given: Value,
    },
    TooSmall {
        min: f64,
        given: f64,
    },
    TooBig {
        max: f64,
        given: f64,
    },
    TooShort {
        min: usize,
        given: usize,
    },
    TooLong {
        max: usize,
        given: usize,
    },
    Unexpected {
        expected: Vec<Value>,
        given: Value,
    },
    BadItem {
        index: usize,
        error: Box<ValueError>,
    },
    BadField {
        field: String,
        error: Box<ValueError>,
    },
    MissingField {
        field: String,
    },
    UnknownField {
        field: String,
    },
    Invalid {
        format: String,
        reason: String,
    },
    Errors {
        list: Vec<ValueError>,
    },
}

impl StdError for ValueError {}

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
            TooSmall { min, given } => {
                "The value should not be less than ".fmt(f)?;
                min.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            TooBig { max, given } => {
                "The value should not be greater than ".fmt(f)?;
                max.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            TooShort { min, given } => {
                "The value should not be shorter than ".fmt(f)?;
                min.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            TooLong { max, given } => {
                "The value should not be longer than ".fmt(f)?;
                max.fmt(f)?;
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            Unexpected { expected, given } => {
                "The value should be one of ".fmt(f)?;
                let mut iter = expected.iter();
                if let Some(value) = iter.next() {
                    value.fmt(f)?;
                    for value in iter {
                        ", ".fmt(f)?;
                        value.fmt(f)?;
                    }
                }
                " but ".fmt(f)?;
                given.fmt(f)?;
                " given".fmt(f)
            }
            BadItem { index, error } => {
                "The item at position ".fmt(f)?;
                index.fmt(f)?;
                " invalid due to: ".fmt(f)?;
                error.fmt(f)
            }
            BadField { field, error } => {
                "The value of field '".fmt(f)?;
                field.fmt(f)?;
                "' invalid due to: ".fmt(f)?;
                error.fmt(f)
            }
            MissingField { field } => {
                "The required field '".fmt(f)?;
                field.fmt(f)?;
                "' is missing".fmt(f)
            }
            UnknownField { field } => {
                "The field '".fmt(f)?;
                field.fmt(f)?;
                "' is unknown".fmt(f)
            }
            Invalid { format, reason } => {
                "The value does not corresponds to ".fmt(f)?;
                format.fmt(f)?;
                if !reason.is_empty() {
                    " due to ".fmt(f)?;
                    reason.fmt(f)?;
                }
                Ok(())
            }
            Errors { list } => {
                let mut iter = list.iter();
                if let Some(error) = iter.next() {
                    error.fmt(f)?;
                    for error in iter {
                        "\n".fmt(f)?;
                        error.fmt(f)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl ValueError {
    pub fn mismatch(expected: &ValueDef, given: &Value) -> Self {
        Self::Mismatch {
            expected: expected.clone(),
            given: given.clone(),
        }
    }

    pub fn too_small(min: f64, given: f64) -> Self {
        Self::TooSmall { min, given }
    }

    pub fn too_big(max: f64, given: f64) -> Self {
        Self::TooBig { max, given }
    }

    pub fn too_short(min: usize, given: usize) -> Self {
        Self::TooShort { min, given }
    }

    pub fn too_long(max: usize, given: usize) -> Self {
        Self::TooLong { max, given }
    }

    pub fn unexpected(expected: &Vec<Value>, given: &Value) -> Self {
        Self::Unexpected {
            expected: expected.clone(),
            given: given.clone(),
        }
    }

    pub fn bad_item(index: usize, error: Self) -> Self {
        Self::BadItem {
            index,
            error: Box::new(error),
        }
    }

    pub fn bad_field(field: impl Into<String>, error: Self) -> Self {
        Self::BadField {
            field: field.into(),
            error: Box::new(error),
        }
    }

    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    pub fn unknown_field(field: impl Into<String>) -> Self {
        Self::UnknownField {
            field: field.into(),
        }
    }

    pub fn invalid(format: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid {
            format: format.into(),
            reason: reason.into(),
        }
    }
}
