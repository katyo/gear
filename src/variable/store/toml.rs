use super::ValueStoreApi;
use crate::{Result, Value};
use toml::Value as TomlValue;

pub struct ValueStore {
    value: TomlValue,
}

impl Default for ValueStore {
    fn default() -> Self {
        Self {
            value: TomlValue::Boolean(false),
        }
    }
}

impl ValueStoreApi for ValueStore {
    fn load(&mut self, data: &[u8]) -> Result<()> {
        self.value = toml::from_slice(data)?;
        Ok(())
    }

    fn save(&self) -> Result<Vec<u8>> {
        Ok(toml::to_vec(&self.value)?)
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

fn lookup<'a>(value: &'a TomlValue, path: &[&str]) -> Option<&'a TomlValue> {
    if path.is_empty() {
        return Some(value);
    }
    if let TomlValue::Table(object) = value {
        object
            .get(path[0])
            .and_then(|value| lookup(value, &path[1..]))
    } else {
        None
    }
}

fn assign(value: &mut TomlValue, path: &[&str], newval: TomlValue) {
    if !matches!(value, TomlValue::Table(_)) {
        *value = TomlValue::Table(Default::default());
    }

    if let TomlValue::Table(object) = value {
        if path.len() > 1 {
            let value = object
                .entry(path[0])
                .or_insert_with(|| TomlValue::Table(Default::default()));
            assign(value, &path[1..], newval);
        } else {
            object.insert(path[0].into(), newval);
        }
    }
}

fn remove(value: &mut TomlValue, path: &[&str]) {
    if let TomlValue::Table(object) = value {
        if path.len() > 1 {
            if let Some(value) = object.get_mut(path[0]) {
                remove(value, &path[1..]);
            }
        } else {
            object.remove(path[0]);
        }
    }
}

fn into(value: &TomlValue) -> Value {
    match value {
        TomlValue::Boolean(value) => Value::Bool(*value),
        TomlValue::Integer(value) => Value::Int(*value),
        TomlValue::Float(value) => Value::Float(*value),
        TomlValue::String(value) => Value::String(value.clone()),
        TomlValue::Datetime(value) => Value::String(value.to_string()),
        TomlValue::Array(value) => Value::List(value.iter().map(into).collect()),
        TomlValue::Table(value) => Value::Dict(
            value
                .iter()
                .map(|(field, value)| (field.clone(), into(value)))
                .collect(),
        ),
    }
}

fn from(value: &Value) -> TomlValue {
    match value {
        Value::None => TomlValue::String("".into()),
        Value::Bool(value) => TomlValue::Boolean(*value),
        Value::Int(value) => TomlValue::Integer(*value),
        Value::Float(value) => TomlValue::Float(*value),
        Value::String(value) => TomlValue::String(value.clone()),
        Value::List(value) => TomlValue::Array(value.iter().map(from).collect()),
        Value::Dict(value) => TomlValue::Table(
            value
                .iter()
                .map(|(field, value)| (field.clone(), from(value)))
                .collect(),
        ),
    }
}
