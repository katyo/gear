use crate::qjs::Error as JsError;
use std::{
    error::Error as StdError,
    ffi::NulError,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError,
    result::Result as StdResult,
    str::Utf8Error,
    string::FromUtf8Error,
};

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Nul(NulError),
    Utf8(Utf8Error),
    Js(JsError),
    App(String),
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Error::Io(error) => {
                "Io Error: ".fmt(f)?;
                error.fmt(f)
            }
            Error::Nul(_) => "Invalid String".fmt(f),
            Error::Utf8(_) => "Invalid Utf8".fmt(f),
            Error::Js(error) => {
                "Js Error: ".fmt(f)?;
                error.fmt(f)
            }
            Error::App(error) => {
                "App Error: ".fmt(f)?;
                error.fmt(f)
            }
        }
    }
}

macro_rules! from_impls {
    ($($type:ty => $variant:ident $($func:ident)*,)*) => {
        $(
            impl From<$type> for Error {
                fn from(error: $type) -> Self {
                    Self::$variant(error$(.$func())*)
                }
            }
        )*
    };
}

from_impls! {
    IoError => Io,
    NulError => Nul,
    Utf8Error => Utf8,
    FromUtf8Error => Utf8 utf8_error,
    JsError => Js,
    String => App,
    &str => App to_string,
}

impl From<Error> for JsError {
    fn from(error: Error) -> JsError {
        match error {
            Error::Io(error) => JsError::IO(error),
            Error::Nul(error) => JsError::InvalidString(error),
            Error::Utf8(error) => JsError::Utf8(error),
            Error::Js(error) => error,
            Error::App(error) => JsError::Exception {
                message: error,
                file: "".into(),
                line: 0,
                stack: "".into(),
            },
        }
    }
}

#[cfg(feature = "watch")]
impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Self {
        use notify::ErrorKind::*;
        match error.kind {
            Generic(error) => Self::App(format!("Notifier error: {}", error)),
            Io(error) => Self::Io(error),
            PathNotFound => Self::App(format!(
                "Notifier path does not exist: {}",
                error
                    .paths
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            WatchNotFound => Self::App("Notifier watch does not exist.".into()),
            InvalidConfig(config) => {
                Self::App(format!("Notifier config is not a valid: {:?}", config))
            }
        }
    }
}
