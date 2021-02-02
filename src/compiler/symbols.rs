use crate::{qjs, Result};
use symbolic_common::{Language, Name, NameMangling};
use symbolic_demangle::{Demangle, DemangleOptions};

#[derive(Debug, Clone, Default)]
pub struct SymbolInfo {
    pub symbol: String,
    pub language: Option<String>,
    pub mangled: Option<bool>,
}

impl From<SymbolInfo> for String {
    fn from(symbol: SymbolInfo) -> String {
        symbol.symbol
    }
}

impl From<&SymbolInfo> for String {
    fn from(symbol: &SymbolInfo) -> String {
        symbol.symbol.clone()
    }
}

impl From<String> for SymbolInfo {
    fn from(symbol: String) -> Self {
        Self {
            symbol,
            ..Default::default()
        }
    }
}

impl From<&str> for SymbolInfo {
    fn from(symbol: &str) -> Self {
        Self {
            symbol: symbol.into(),
            ..Default::default()
        }
    }
}

#[derive(qjs::FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct DemangleOpts {
    #[quickjs(default = "default_true")]
    return_type: bool,
    #[quickjs(default = "default_true")]
    parameters: bool,
}

impl Default for DemangleOpts {
    fn default() -> Self {
        Self {
            return_type: default_true(),
            parameters: default_true(),
        }
    }
}

const fn default_true() -> bool {
    true
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
pub mod js {
    pub use super::*;

    impl SymbolInfo {
        #[quickjs(rename = "new")]
        pub fn new_js0(
            symbol: String,
            language: qjs::Opt<String>,
            mangled: qjs::Opt<bool>,
        ) -> Self {
            Self {
                symbol,
                language: language.0,
                mangled: mangled.0,
            }
        }

        #[quickjs(rename = "new")]
        pub fn new_js1(
            symbol: String,
            mangled: qjs::Opt<bool>,
            language: qjs::Opt<String>,
        ) -> Self {
            Self {
                symbol,
                language: language.0,
                mangled: mangled.0,
            }
        }

        #[quickjs(rename = "new")]
        pub fn new_js2<'js>(data: qjs::Object<'js>) -> qjs::Result<Self> {
            Ok(Self {
                symbol: data.get("symbol")?,
                language: data.get("language")?,
                mangled: data.get("mangled")?,
            })
        }

        #[quickjs(get, enumerable)]
        pub fn symbol(&self) -> &String {
            &self.symbol
        }

        #[quickjs(set, rename = "symbol")]
        pub fn set_symbol(&mut self, symbol: String) {
            self.symbol = symbol;
        }

        #[quickjs(get, enumerable)]
        pub fn language(&self) -> &Option<String> {
            &self.language
        }

        #[quickjs(set, rename = "language")]
        pub fn set_language(&mut self, language: Option<String>) {
            self.language = language;
        }

        #[quickjs(get, enumerable)]
        pub fn mangled(&self) -> &Option<bool> {
            &self.mangled
        }

        #[quickjs(set, rename = "mangled")]
        pub fn set_mangled(&mut self, mangled: Option<bool>) {
            self.mangled = mangled;
        }

        pub fn demangle(&self, opts: qjs::Opt<DemangleOpts>) -> Result<Self> {
            let mangling = match &self.mangled {
                None => NameMangling::Unknown,
                Some(true) => NameMangling::Mangled,
                Some(false) => Err(format!("Symbol already demangled"))?,
            };
            let language = match &self.language {
                None => Language::Unknown,
                Some(name) => name
                    .parse()
                    .map_err(|_| format!("Unsupported lanuage `{}`", name))?,
            };
            let opts = opts.0.unwrap_or_default();
            let opts = DemangleOptions::name_only()
                .return_type(opts.return_type)
                .parameters(opts.parameters);
            let name = Name::new(&self.symbol, mangling, language);
            let de_symbol = name
                .demangle(opts)
                .ok_or_else(|| format!("Unable to demangle symbol"))?;
            let de_language = name.detect_language();
            if language != Language::Unknown && language != de_language {
                Err(format!("Language is differs"))?;
            }
            Ok(Self {
                symbol: de_symbol.to_string(),
                language: Some(de_language.to_string()),
                mangled: Some(false),
            })
        }
    }
}
