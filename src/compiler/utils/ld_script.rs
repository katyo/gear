use crate::{qjs, Map, Set};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, Default, qjs::FromJs)]
pub struct LdScript {
    pub entry: Option<String>,
    #[quickjs(default)]
    pub memory: Map<String, LdRegion>,
    #[quickjs(default)]
    pub externs: Set<String>,
    #[quickjs(default)]
    pub provides: Map<String, LdProvideExpr>,
    #[quickjs(default)]
    pub sections: Vec<String>,
    #[quickjs(default)]
    pub includes: Set<String>,
}

impl Display for LdScript {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        if !self.memory.is_empty() {
            writeln!(f, "MEMORY {{")?;
            for (name, region) in &self.memory {
                writeln!(f, "    {} {}", name, region)?;
            }
            writeln!(f, "}}")?;
        }
        if let Some(entry) = &self.entry {
            writeln!(f, "ENTRY({});", entry)?;
        }
        for name in &self.externs {
            writeln!(f, "EXTERN({});", name)?;
        }
        for (name, provide) in &self.provides {
            writeln!(f, "PROVIDE({} = {});", name, provide)?;
        }
        for name in &self.includes {
            writeln!(f, "INCLUDE {}", name)?;
        }
        if !self.sections.is_empty() {
            writeln!(f, "SECTIONS {{")?;
            for section in &self.sections {
                section.fmt(f)?;
            }
            writeln!(f, "}}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, qjs::FromJs)]
pub struct LdRegion {
    pub address: u64,
    pub size: u64,
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl Display for LdRegion {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        writeln!(
            f,
            "({}{}{}) : ORIGIN = 0x{:x}, LENGTH = 0x{:x}",
            if self.read { "r" } else { "" },
            if self.write { "w" } else { "" },
            if self.exec { "x" } else { "" },
            self.address,
            self.size
        )
    }
}

#[derive(Debug, Clone, qjs::FromJs)]
#[quickjs(rename_all = "lowercase")]
pub enum LdProvideExpr {
    Int(i64),
    Neg(Box<Self>),
    Sum(Vec<Self>),
    Start(String),
    End(String),
    Size(String),
}

impl Display for LdProvideExpr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Int(val) => val.fmt(f),
            Self::Neg(nest) => write!(f, "-({})", nest),
            Self::Sum(nest) => {
                let mut iter = nest.iter();
                if let Some(nest) = iter.next() {
                    nest.fmt(f)?;
                    for nest in iter {
                        " + ".fmt(f)?;
                        nest.fmt(f)?;
                    }
                } else {
                    0.fmt(f)?;
                }
                Ok(())
            }
            Self::Start(name) => write!(f, "ORIGIN({})", name),
            Self::End(name) => write!(f, "ORIGIN({}) + LENGTH({})", name, name),
            Self::Size(name) => write!(f, "LENGTH({})", name),
        }
    }
}

/*
#[derive(Debug, Clone, qjs::FromJs)]
#[quickjs(tag = "op", rename_all = "lowercase")]
pub enum LdSectionStmt {
    Set {
        #[quickjs(default)]
        var: LdSectionVar,
        expr: LdSectionExpr,
    },
    Inc {
        #[quickjs(default)]
        var: LdSectionVar,
        expr: LdSectionExpr,
    },
}

#[derive(Debug, Clone, qjs::FromJs)]
#[quickjs(untagged)]
pub enum LdSectionVar {
    /// The `.` (dot)
    Location,
    /// The symbol name
    Symbol(String),
}

#[derive(Debug, Clone, qjs::FromJs)]
pub enum LdSectionExpr {
    Int(i64),
    Var(LdSectionVar),
    Neg(Box<Self>),
    Sum(Vec<Self>),
}
*/
