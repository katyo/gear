use super::{Value, ValueDef, ValueError, ValueResult};
use std::result::Result as StdResult;

impl Value {
    pub fn check(&self, def: &ValueDef) -> ValueResult<()> {
        def.check(self)
    }
}

impl ValueDef {
    pub fn check(&self, value: &Value) -> ValueResult<()> {
        match self {
            ValueDef::Any {} => Ok(()),
            ValueDef::Bool {} => match value {
                Value::Bool(_) => Ok(()),
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::Int { min, max } => match value {
                Value::Int(given) => {
                    if given < min {
                        Err(ValueError::too_small(*min as _, *given as _))
                    } else if given > max {
                        Err(ValueError::too_big(*max as _, *given as _))
                    } else {
                        Ok(())
                    }
                }
                Value::Float(given) if given.fract() == 0.0 => {
                    let given = *given as i64;
                    if &given < min {
                        Err(ValueError::too_small(*min as _, given as _))
                    } else if &given > max {
                        Err(ValueError::too_big(*max as _, given as _))
                    } else {
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::Float { min, max } => match value {
                Value::Float(given) => {
                    if given < min {
                        Err(ValueError::too_small(*min, *given))
                    } else if given > max {
                        Err(ValueError::too_big(*max, *given))
                    } else {
                        Ok(())
                    }
                }
                Value::Int(given) => {
                    let given = *given as f64;
                    if &given < min {
                        Err(ValueError::too_small(*min, given))
                    } else if &given > max {
                        Err(ValueError::too_big(*max, given))
                    } else {
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::String { min, max } => match value {
                Value::String(val) => {
                    let len = val.len();
                    if &len < min {
                        Err(ValueError::too_short(*min, len))
                    } else if &len > max {
                        Err(ValueError::too_long(*max, len))
                    } else {
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::Option { value: expected } => match value {
                &Value::None => Ok(()),
                given => expected.check(given),
            },
            ValueDef::Either { options } => {
                if let Ok(list) = options
                    .iter()
                    .map(|variant| {
                        if let Err(error) = value.check(variant) {
                            Ok(error)
                        } else {
                            Err(())
                        }
                    })
                    .collect::<StdResult<Vec<_>, ()>>()
                {
                    Err(ValueError::Errors { list })
                } else {
                    Ok(())
                }
            }
            ValueDef::Enum {
                value: expected,
                options,
            } => {
                expected.check(value)?;
                for option in options {
                    if option == value {
                        return Ok(());
                    }
                }
                Err(ValueError::unexpected(options, value))
            }
            ValueDef::Tuple { values } => match value {
                Value::List(given) => {
                    let values_len = values.len();
                    let given_len = given.len();
                    if given_len < values_len {
                        Err(ValueError::too_short(values_len, given_len))
                    } else if given_len > values_len {
                        Err(ValueError::too_long(values_len, given_len))
                    } else {
                        for index in 0..given_len {
                            values[index]
                                .check(&given[index])
                                .map_err(|error| ValueError::bad_item(index, error))?;
                        }
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::Record { fields } => match value {
                Value::Dict(given) => {
                    for (field, expected) in fields {
                        let value = given.get(field).unwrap_or(&Value::None);
                        expected.check(value).map_err(|error| {
                            if !given.contains_key(field) {
                                ValueError::missing_field(field)
                            } else {
                                error
                            }
                        })?;
                    }
                    for field in given.keys() {
                        if !fields.contains_key(field) {
                            return Err(ValueError::unknown_field(field));
                        }
                    }
                    Ok(())
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::List {
                value: expected,
                min,
                max,
            } => match value {
                Value::List(given) => {
                    let given_len = given.len();
                    if &given_len < min {
                        Err(ValueError::too_short(*min, given_len))
                    } else if &given_len > max {
                        Err(ValueError::too_long(*max, given_len))
                    } else {
                        for index in 0..given_len {
                            if let Err(error) = expected.check(&given[index]) {
                                return Err(ValueError::bad_item(index, error));
                            }
                        }
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
            ValueDef::Dict {
                value: expected,
                min,
                max,
            } => match value {
                Value::Dict(given) => {
                    let given_len = given.len();
                    if &given_len < min {
                        Err(ValueError::too_short(*min, given_len))
                    } else if &given_len > max {
                        Err(ValueError::too_long(*max, given_len))
                    } else {
                        for (field, value) in given {
                            if let Err(error) = expected.check(value) {
                                return Err(ValueError::bad_field(field, error));
                            }
                        }
                        Ok(())
                    }
                }
                given => Err(ValueError::mismatch(self, given)),
            },
        }
    }
}
