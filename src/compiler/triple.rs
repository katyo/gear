macro_rules! enum_impl {
	  ($( $(#[$typemeta:meta])* $type:ident { $(#[$defvarmeta:meta])* $defvar:ident => $defname:literal $($defaltname:literal)*, $($(#[$varmeta:meta])* $var:ident $(($subtype:ident::$defsubvar:ident))* => $name:literal $($altname:literal)*,)* $({  })* } $(($parseinput:ident) { $($parsebody:tt)* })* )*) => {
        $(
            $(#[$typemeta])*
		        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, crate::qjs::FromJs, crate::qjs::IntoJs)]
            #[quickjs(untagged)]
            #[repr(u32)]
            #[quickjs(rename_all = "lowercase")]
            pub enum $type {
                $(#[$defvarmeta])*
                $defvar,

                $(
                    $(#[$varmeta])*
                    $var $(($subtype))?,
                )*
            }

            impl Default for $type {
                fn default() -> Self {
                    Self::$defvar
                }
            }

            impl std::fmt::Display for $type {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    match self {
                        $(Self::$var $((enum_impl!(@value val $subtype)))* => {
                            $name.fmt(f)?;
                            $(enum_impl!(@value val $subtype).fmt(f)?;)*
                        },)*
                        _ => $defname.fmt(f)?,
                    }
                    Ok(())
                }
            }

            impl std::str::FromStr for $type {
                type Err = ();

                fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                    Ok(match s {
                        $($name $(| $altname)* => Self::$var $(($subtype::$defsubvar))?,)*
                        _ => {
                            $(if let Ok(this) = {
                                let $parseinput = s;
                                $($parsebody)*
                            } {
                                return Ok(this);
                            })*
                            Self::$defvar
                        },
                    })
                }
            }
        )*
	  };

    (@value $value:ident $dummy:ident) => { $value };
}

mod arch;
mod env;
mod format;
mod os;
mod vendor;

pub use arch::*;
pub use env::*;
pub use format::*;
pub use os::*;
pub use vendor::*;

use crate::qjs;
use std::{fmt, str};

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct Triple {
    pub arch: Arch,
    pub vendor: Vendor,
    pub os: Os,
    pub env: Env,
    pub format: ObjFmt,
}

impl Triple {
    pub fn new(arch: Arch, vendor: Vendor, os: Os, env: Env, format: ObjFmt) -> Self {
        Self {
            arch,
            vendor,
            os,
            env,
            format,
        }
        //.set_defaults()
    }

    fn default_format(&self) -> ObjFmt {
        match self.arch {
            Arch::Unknown
            | Arch::AArch64(_)
            | Arch::AArch64_32(_)
            | Arch::Arm(_)
            | Arch::Thumb(_)
            | Arch::X86
            | Arch::X86_64 => match self.os {
                Os::Darwin => ObjFmt::MachO,
                Os::Win32 => ObjFmt::COFF,
                _ => ObjFmt::ELF,
            },
            Arch::Ppc | Arch::Ppc64 => {
                if self.os == Os::AIX {
                    ObjFmt::XCOFF
                } else {
                    ObjFmt::ELF
                }
            }
            Arch::SystemZ => {
                if self.os == Os::ZOS {
                    ObjFmt::GOFF
                } else {
                    ObjFmt::ELF
                }
            }
            Arch::Wasm32 | Arch::Wasm64 => ObjFmt::Wasm,
            _ => ObjFmt::ELF,
        }
    }

    fn set_defaults(self) -> Self {
        if self.format == ObjFmt::Unknown {
            self.format = self.default_format();
        }
        self
    }
}

impl str::FromStr for Triple {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut p = s.split('-');
        match (p.next(), p.next(), p.next(), p.next(), p.next()) {
            (Some(a), Some(b), Some(c), Some(d), Some(e)) => Ok(Self::new(
                Arch::from_str(a)?,
                Vendor::from_str(b)?,
                Os::from_str(c)?,
                Env::from_str(d)?,
                ObjFmt::from_str(e)?,
            )),
            (Some(a), Some(b), Some(c), Some(d), None) => Ok(Self::new(
                Arch::from_str(a)?,
                Vendor::from_str(b)?,
                Os::from_str(c)?,
                Env::from_str(d)?,
                ObjFmt::Unknown,
            )),
            (Some(a), Some(b), Some(c), None, None) => Ok(Self::new(
                Arch::from_str(a)?,
                Vendor::Unknown,
                Os::from_str(b)?,
                Env::from_str(c)?,
                ObjFmt::Unknown,
            )),
            (Some(arch), Some(format), None, None, None) => Ok(Self::new(
                Arch::from_str(arch)?,
                Vendor::Unknown,
                Os::Unknown,
                Env::Unknown,
                ObjFmt::from_str(format)?,
            )),
            _ => Ok(Default::default()),
        }
    }
}
