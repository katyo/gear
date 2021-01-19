mod check;
mod definition;
mod result;
mod store;
mod validation;
mod value;

pub use definition::{ValueDef, VariableDef};
pub use result::{ValueError, ValueResult};
pub use store::ValueStore;
pub use validation::Validator;
pub use value::Value;

use crate::{qjs, Map, Mut, Ref, Result, Weak, WeakElement, WeakKey, WeakSet};

use std::{
    borrow::Borrow,
    fmt,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct Variable(Ref<Internal>);

impl Variable {
    pub fn weak(&self) -> WeakVariable {
        WeakVariable(Ref::downgrade(&self.0))
    }

    pub fn fmt_tree(&self, ident: usize, f: &mut Formatter) -> FmtResult {
        let spaces = ident * 4;
        write!(f, "{:ident$}{}", "", self.name(), ident = spaces)?;
        ": ".fmt(f)?;
        self.definition().fmt(f)?;
        " = ".fmt(f)?;
        self.value().fmt(f)?;
        let text = self.description();
        if !text.is_empty() {
            " // ".fmt(f)?;
            text.fmt(f)?;
        }
        '\n'.fmt(f)
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "Var `".fmt(f)?;
        self.name().fmt(f)?;
        "`: ".fmt(f)?;
        self.definition().fmt(f)?;
        " = ".fmt(f)?;
        self.value().fmt(f)
    }
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Display::fmt(self, f)
    }
}

impl Borrow<str> for Variable {
    fn borrow(&self) -> &str {
        self.0.as_ref().borrow()
    }
}

impl Borrow<String> for Variable {
    fn borrow(&self) -> &String {
        self.0.as_ref().borrow()
    }
}

impl PartialEq<Variable> for Variable {
    fn eq(&self, other: &Variable) -> bool {
        self.0 == other.0
    }
}

impl Eq for Variable {}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

pub struct Internal {
    def: VariableDef,
    //validator: Option<Box<dyn Validator + Send + Sync>>,
    value: Mut<Value>,
}

impl Drop for Internal {
    fn drop(&mut self) {
        log::debug!("Variable::drop `{}`", self.def.name);
    }
}

impl Borrow<str> for Internal {
    fn borrow(&self) -> &str {
        &self.def.name
    }
}

impl Borrow<String> for Internal {
    fn borrow(&self) -> &String {
        &self.def.name
    }
}

impl PartialEq for Internal {
    fn eq(&self, other: &Self) -> bool {
        self.def.name == other.def.name
    }
}

impl Eq for Internal {}

impl Hash for Internal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.def.name.hash(state);
    }
}

impl From<VariableDef> for Variable {
    fn from(def: VariableDef) -> Self {
        log::debug!("Variable::new `{}`", def.name);
        let value = Mut::new(def.default.clone());
        Self(Ref::new(Internal { def, value }))
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct WeakVariable(Weak<Internal>);

impl WeakVariable {
    pub fn try_ref(&self) -> Option<Variable> {
        self.0.upgrade().map(Variable)
    }
}

impl WeakKey for WeakVariable {
    type Key = Internal;

    fn with_key<F, R>(view: &Self::Strong, f: F) -> R
    where
        F: FnOnce(&Self::Key) -> R,
    {
        f(&view.0)
    }
}

impl WeakElement for WeakVariable {
    type Strong = Variable;

    fn new(view: &Self::Strong) -> Self {
        view.weak()
    }

    fn view(&self) -> Option<Self::Strong> {
        self.try_ref()
    }

    fn clone(view: &Self::Strong) -> Self::Strong {
        view.clone()
    }
}

pub type WeakVariableSet = WeakSet<WeakVariable>;

struct StoreInternal {
    values: Mut<ValueStore>,
    args: Map<String, String>,
    variables: Mut<WeakVariableSet>,
}

#[derive(Clone)]
pub struct VariableStore(Ref<StoreInternal>);

impl VariableStore {
    pub fn new(values: ValueStore, args: impl Iterator<Item = (String, String)>) -> Self {
        Self(Ref::new(StoreInternal {
            values: Mut::new(values),
            args: args.collect(),
            variables: Default::default(),
        }))
    }

    pub fn reset(&self) {
        *self.0.variables.write() = Default::default();
    }

    pub fn new_variable(
        &self,
        name: impl AsRef<str>,
        description: impl Into<String>,
        definition: Option<ValueDef>,
        default: Option<Value>,
    ) -> Result<Variable> {
        let name = name.as_ref();
        {
            if self.0.variables.read().contains(name) {
                return Err(format!("Variable `{}` already exists", name).into());
            }
        }

        let variable = Variable::from(VariableDef::new(name, description, definition, default));

        if let Some(value) = &self.0.values.read().get(variable.name()) {
            if let Err(error) = value.check(&variable.definition()) {
                log::warn!(
                    "Attempt to use bad value `{}` for variable `{}` due to: {}",
                    value,
                    variable.name(),
                    error
                );
            } else {
                //value.coerce(&def.definition)
                variable.set_value(value.clone());
            }
        }

        if let Some(value) = self.0.args.get(variable.name()) {
            match serde_json::from_str::<Value>(&value) {
                Ok(value) => {
                    if let Err(error) = value.check(&variable.definition()) {
                        log::warn!(
                            "Attempt to use bad value `{}` for variable `{}` due to: {}",
                            value,
                            variable.name(),
                            error
                        );
                    } else {
                        //value.coerce(&def.definition)
                        variable.set_value(value.clone());
                    }
                }
                Err(error) => {
                    log::warn!(
                        "Error when parsing values `{}` for variable `{}` due to: {}",
                        value,
                        variable.name(),
                        error
                    );
                }
            }
        }

        self.0.variables.write().insert(variable.clone());

        Ok(variable)
    }

    /*pub fn unused_values(&self) -> impl Iterator<Item = String> {
        self.0.values.read().iter().map(||)
    }*/
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    pub use super::*;

    impl Variable {
        pub fn new() -> Self {
            unimplemented!();
        }

        #[quickjs(get, enumerable)]
        pub fn name(&self) -> &str {
            &self.0.def.name
        }

        #[quickjs(get, enumerable)]
        pub fn description(&self) -> &str {
            &self.0.def.description
        }

        #[quickjs(get, enumerable)]
        pub fn default(&self) -> &Value {
            &self.0.def.default
        }

        #[quickjs(get, enumerable)]
        pub fn definition(&self) -> &ValueDef {
            &self.0.def.definition
        }

        #[quickjs(get, enumerable)]
        pub fn value(&self) -> Value {
            self.0.value.read().clone()
        }

        #[quickjs(set, rename = "value")]
        pub fn set_value(&self, value: Value) {
            *self.0.value.write() = value;
        }

        #[quickjs(rename = "toString")]
        pub fn to_string_js(&self) -> String {
            self.to_string()
        }
    }
}
