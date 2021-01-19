use super::ValueStoreApi;
use crate::{Result, Value};
use serde_json::Value as JsonValue;

pub struct ValueStore {
    value: JsonValue,
}

impl Default for ValueStore {
    fn default() -> Self {
        Self {
            value: JsonValue::default(),
        }
    }
}

impl ValueStoreApi for ValueStore {
    fn load(&mut self, data: &[u8]) -> Result<()> {
        self.value = serde_json::from_slice(data)?;
        Ok(())
    }

    fn save(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(&self.value)?)
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

fn lookup<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
    if path.is_empty() {
        return Some(value);
    }
    if let JsonValue::Object(object) = value {
        object
            .get(path[0])
            .and_then(|value| lookup(value, &path[1..]))
    } else {
        None
    }
}

fn assign(value: &mut JsonValue, path: &[&str], newval: JsonValue) {
    if !matches!(value, JsonValue::Object(_)) {
        *value = JsonValue::Object(Default::default());
    }

    if let JsonValue::Object(object) = value {
        if path.len() > 1 {
            let value = object
                .entry(path[0])
                .or_insert_with(|| JsonValue::Object(Default::default()));
            assign(value, &path[1..], newval);
        } else {
            object.insert(path[0].into(), newval);
        }
    }
}

fn remove(value: &mut JsonValue, path: &[&str]) {
    if let JsonValue::Object(object) = value {
        if path.len() > 1 {
            if let Some(value) = object.get_mut(path[0]) {
                remove(value, &path[1..]);
            }
        } else {
            object.remove(path[0]);
        }
    }
}

fn into(value: &JsonValue) -> Value {
    match value {
        JsonValue::Null => Value::None,
        JsonValue::Bool(value) => Value::Bool(*value),
        JsonValue::Number(value) => value.as_i64().map(Value::Int).unwrap_or_else(|| {
            value.as_f64().map(Value::Float).unwrap_or_else(|| {
                value
                    .as_u64()
                    .map(|value| Value::Float(value as _))
                    .unwrap()
            })
        }),
        JsonValue::String(value) => Value::String(value.clone()),
        JsonValue::Array(value) => Value::List(value.iter().map(into).collect()),
        JsonValue::Object(value) => Value::Dict(
            value
                .iter()
                .map(|(field, value)| (field.clone(), into(value)))
                .collect(),
        ),
    }
}

fn from(value: &Value) -> JsonValue {
    match value {
        Value::None => JsonValue::Null,
        Value::Bool(value) => JsonValue::Bool(*value),
        Value::Int(value) => JsonValue::Number((*value).into()),
        Value::Float(value) => {
            JsonValue::Number(serde_json::Number::from_f64(*value).unwrap_or(0.into()))
        }
        Value::String(value) => JsonValue::String(value.clone()),
        Value::List(value) => JsonValue::Array(value.iter().map(from).collect()),
        Value::Dict(value) => JsonValue::Object(
            value
                .iter()
                .map(|(field, value)| (field.clone(), from(value)))
                .collect(),
        ),
    }
}
