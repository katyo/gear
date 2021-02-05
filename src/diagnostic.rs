use crate::qjs;
use std::{
    fmt,
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
    str::FromStr,
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn severity(&self) -> Severity {
        self.0.iter().fold(
            Severity::Debug,
            |min_severity, Diagnostic { severity, .. }| min_severity.min(*severity),
        )
    }

    pub fn is_failed(&self) -> bool {
        self.severity() <= Severity::Error
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub locations: Vec<Location>,
    pub children: Diagnostics,
    pub fixits: Vec<FixingSuggestion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, qjs::FromJs, qjs::IntoJs)]
#[quickjs(untagged, rename_all = "lowercase")]
#[repr(u8)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
    Note,
    Debug,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Fatal
    }
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fatal => "fatal",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Debug => "debug",
        }
    }
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.as_str().fmt(f)
    }
}

impl FromStr for Severity {
    type Err = ();

    fn from_str(input: &str) -> StdResult<Self, ()> {
        Ok(match input {
            "fatal error" | "internal compiler error" | "sorry, unimplemented" => Self::Fatal,
            "error" => Self::Error,
            "warning" | "anachronism" => Self::Warning,
            "remark" | "note" => Self::Note,
            "debug" => Self::Debug,
            _ => {
                if input.contains("fatal")
                    || input.contains("internal")
                    || input.contains("unimplement")
                {
                    Self::Fatal
                } else if input.contains("error") {
                    Self::Error
                } else if input.contains("warn") {
                    Self::Warning
                } else {
                    Self::Fatal
                }
            }
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct Location {
    pub file: String,
    pub span: Option<TextSpan>,
    pub point: Option<TextPoint>,
    pub label: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct FixingSuggestion {
    pub file: String,
    pub span: TextSpan,
    pub text: String,
}

impl Display for FixingSuggestion {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        "fix-it:".fmt(f)?;
        fmt::Debug::fmt(&self.file, f)?;
        ":{".fmt(f)?;
        self.span.fmt(f)?;
        "}:".fmt(f)?;
        fmt::Debug::fmt(&self.text, f)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct TextSpan {
    pub start: TextPoint,
    pub end: TextPoint,
}

impl Display for TextSpan {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.start.fmt(f)?;
        '-'.fmt(f)?;
        self.end.fmt(f)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct TextPoint {
    pub line: u32,
    pub column: u32,
}

impl Display for TextPoint {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.line.fmt(f)?;
        ':'.fmt(f)?;
        self.column.fmt(f)
    }
}
