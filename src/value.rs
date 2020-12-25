use crate::{Map, Result};

pub enum Type {
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
    Either {
        left: Type,
        right: Type,
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
        items: Map<String, RecItem>,
    },
}

pub struct RecItem {
    type_: Type,
    opt: bool,
}

pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
    Tuple(Vec<Value>),
    List(Vec<Value>),
    Dict(Map<String, Value>),
}

/*impl Value {
    pub fn check(&self, type_: &Type) -> Result<()> {
        match type_ {
            Type::Bool => match self {},
        }
    }
}*/

/*pub struct Value {
    type_: Type,
    value:
}*/
