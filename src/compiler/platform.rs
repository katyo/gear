use crate::{qjs, Result};
use derive_deref::{Deref, DerefMut};
use semver::Version;
use std::{
    //fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut)]
pub struct SemVer(Version);

impl<'js> qjs::FromJs<'js> for SemVer {
    fn from_js(ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        Ok(Self(match val.type_of() {
            qjs::Type::Int => {
                let m: u64 = val.get()?;
                Version::new(m, 0, 0)
            }
            qjs::Type::String => {
                let s: String = val.get()?;
                Version::parse(&s).map_err(|error| {
                    qjs::Error::new_from_js_message("string", "semver", error.to_string())
                })?
            }
            qjs::Type::Array => {
                let v: Vec<u64> = val.get()?;
                let l = v.len();
                Version::new(
                    if l > 0 { v[0] } else { 0 },
                    if l > 1 { v[1] } else { 0 },
                    if l > 2 { v[2] } else { 0 },
                )
            }
            ty => {
                return Err(qjs::Error::new_from_js(ty.as_str(), "semver"));
            }
        }))
    }
}

impl<'js> qjs::IntoJs<'js> for SemVer {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.0.to_string().into_js(ctx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
#[quickjs(tag = "type", rename_all = "lowercase")]
pub enum FileKind {
    Executable,
    Dynamic {
        library: bool,
        version: Option<SemVer>,
    },
    Static {
        library: bool,
    },
    Object,
}

impl FileKind {
    pub fn file_name(&self, platform: &PlatformKind, name: impl AsRef<str>) -> String {
        /*version.iter().fold(String::default(), |mut out, num| {
            use std::fmt::Write;
            write!(&mut out, ".{}", num);
            out
        })*/

        let name = name.as_ref();
        let (prefix, suffix, version) = match platform {
            PlatformKind::None | PlatformKind::Unix => match self {
                Self::Executable => ("", "", None),
                Self::Dynamic { library, version } => {
                    (if *library { "lib" } else { "" }, ".so", version.as_ref())
                }
                Self::Static { library } => (if *library { "lib" } else { "" }, ".a", None),
                Self::Object => ("", ".o", None),
            },
            PlatformKind::Darwin => match self {
                Self::Executable => ("", "", None),
                Self::Dynamic { library, version } => (
                    if *library { "lib" } else { "" },
                    ".dylib",
                    version.as_ref(),
                ),
                Self::Static { library } => (if *library { "lib" } else { "" }, ".a", None),
                Self::Object => ("", ".o", None),
            },
            PlatformKind::Windows => match self {
                Self::Executable => ("", ".exe", None),
                Self::Dynamic { library, .. } => (if *library { "lib" } else { "" }, ".dll", None),
                Self::Static { library } => (if *library { "lib" } else { "" }, ".lib", None),
                Self::Object => ("", ".obj", None),
            },
        };

        if let Some(version) = version {
            format!("{}{}{}.{}", prefix, name, suffix, version.0)
        } else {
            format!("{}{}{}", prefix, name, suffix)
        }
    }
}

/*impl FromStr for FileKind {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        Ok(match s {
            "so" | "dylib" | "dll" => Self::Dynamic,
            "a" | "lib" => Self::Static,
            "o" | "obj" => Self::Object,
            "elf" | "exe" | "bin" | "" => Self::Executable,
            _ => return Err(()),
        })
    }
}*/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformKind {
    None,
    Unix,
    Darwin,
    Windows,
}

impl PlatformKind {
    pub fn from_target(target: impl AsRef<str>) -> Result<Self> {
        let target = target.as_ref();
        if target.contains("-none-") {
            Ok(Self::None)
        } else if target.contains("-windows-") {
            Ok(Self::Windows)
        } else if target.contains("-apple-") {
            Ok(Self::Darwin)
        } else if target.contains("bsd-") || target.contains("-linux-") || target.contains("-hurd-")
        {
            Ok(Self::Unix)
        } else {
            Err(format!("Unable to determine platform for target `{}`", target).into())
        }
    }
}

impl FromStr for PlatformKind {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        Ok(match s {
            "macos" | "osx" => Self::Darwin,
            "windows" => Self::Windows,
            "freebsd" | "openbsd" | "netbsd" | "solaris" | "linux" | "hurd" => Self::Unix,
            _ => return Err(()),
        })
    }
}
