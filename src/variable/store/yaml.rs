use super::ValueStoreApi;
use crate::{Result, Value};
use serde_yaml::Value as YamlValue;

pub struct ValueStore {
    value: YamlValue,
}

impl Default for ValueStore {
    fn default() -> Self {
        Self {
            value: YamlValue::default(),
        }
    }
}

impl ValueStoreApi for ValueStore {
    fn load(&mut self, data: &[u8]) -> Result<()> {
        self.value = serde_yaml::from_slice(data)?;
        Ok(())
    }

    fn save(&self) -> Result<Vec<u8>> {
        Ok(serde_yaml::to_vec(&self.value)?)
    }

    fn get(&self, path: &[&str]) -> Option<Value> {
        lookup(&self.value, path).map(into)
    }

    fn set(&mut self, path: &[&str], value: Option<&Value>) {
        if let Some(value) = value {
            assign(&mut self.value, path, from(value));
        } else {
            remove(&mut self.value, path);
        }
    }
}

fn lookup<'a>(value: &'a YamlValue, path: &[&str]) -> Option<&'a YamlValue> {
    if path.is_empty() {
        return Some(value);
    }
    if let YamlValue::Mapping(object) = value {
        object
            .get(&YamlValue::String(path[0].into()))
            .and_then(|value| lookup(value, &path[1..]))
    } else {
        None
    }
}

fn assign(value: &mut YamlValue, path: &[&str], newval: YamlValue) {
    if !matches!(value, YamlValue::Mapping(_)) {
        *value = YamlValue::Mapping(Default::default());
    }

    if let YamlValue::Mapping(object) = value {
        if path.len() > 1 {
            let name = YamlValue::String(path[0].into());
            if !object.contains_key(&name) {
                object.insert(name.clone(), YamlValue::Mapping(Default::default()));
            }
            if let Some(value) = object.get_mut(&name) {
                assign(value, &path[1..], newval);
            }
        } else {
            object.insert(path[0].into(), newval);
        }
    }
}

fn remove(value: &mut YamlValue, path: &[&str]) {
    if let YamlValue::Mapping(object) = value {
        let name = YamlValue::String(path[0].into());
        if path.len() > 1 {
            if let Some(value) = object.get_mut(&name) {
                remove(value, &path[1..]);
            }
        } else {
            object.remove(&name);
        }
    }
}

fn into(value: &YamlValue) -> Value {
    match value {
        YamlValue::Null => Value::None,
        YamlValue::Bool(value) => Value::Bool(*value),
        YamlValue::Number(value) => value.as_i64().map(Value::Int).unwrap_or_else(|| {
            value.as_f64().map(Value::Float).unwrap_or_else(|| {
                value
                    .as_u64()
                    .map(|value| Value::Float(value as _))
                    .unwrap()
            })
        }),
        YamlValue::String(value) => Value::String(value.clone()),
        YamlValue::Sequence(value) => Value::List(value.iter().map(into).collect()),
        YamlValue::Mapping(value) => Value::Dict(
            value
                .iter()
                .map(|(field, value)| (into_key(field), into(value)))
                .collect(),
        ),
    }
}

fn from(value: &Value) -> YamlValue {
    match value {
        Value::None => YamlValue::Null,
        Value::Bool(value) => YamlValue::Bool(*value),
        Value::Int(value) => YamlValue::Number((*value).into()),
        Value::Float(value) => YamlValue::Number((*value).into()),
        Value::String(value) => YamlValue::String(value.clone()),
        Value::List(value) => YamlValue::Sequence(value.iter().map(from).collect()),
        Value::Dict(value) => YamlValue::Mapping(
            value
                .iter()
                .map(|(field, value)| (from_key(field), from(value)))
                .collect(),
        ),
    }
}

fn into_key(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "null".into(),
        YamlValue::Bool(value) => value.to_string(),
        YamlValue::Number(value) => value.to_string(),
        YamlValue::String(value) => value.clone(),
        YamlValue::Sequence(value) => value.iter().map(into_key).collect::<Vec<_>>().join(","),
        YamlValue::Mapping(value) => value
            .iter()
            .map(|(field, value)| format!("{}:{}", into_key(field), into_key(value)))
            .collect::<Vec<_>>()
            .join(","),
    }
}

fn from_key(value: &str) -> YamlValue {
    YamlValue::String(value.into())
}
