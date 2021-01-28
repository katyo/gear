enum_impl! {
    /// Object Format
    ObjFmt {
        /// Unknown format
        Unknown => "unknown",

        /// GOFF (IBM OS/360)
        GOFF => "goff",
        /// COFF Common Object File Format (Unix System V R4, Windows)
        COFF => "coff",
        /// ELF Executable and Linkable Format
        ELF => "elf",
        /// MACHO (NeXT, Apple MacOSX, iOS, ...)
        MachO => "macho",
        /// Wasm
        Wasm => "wasm",
        /// XCOFF (IBM AIX, BeOS, MacOS, )
        XCOFF => "xcoff",
    }
}

impl ObjFmt {
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::COFF => &["", "o", "obj"],
            Self::ELF => &[
                "", "axf", "bin", "elf", "o", "prx", "puff", "ko", "mod", "so",
            ],
            Self::MachO => &["", "o", "dylib", "bundle"],
            _ => &[],
        }
    }

    pub fn magic(&self) -> &'static [u8] {
        match self {
            Self::ELF => &[0x7f, b'E', b'L', b'F'],
            Self::MachO => &[0xfe, 0xed, 0xfa],
            Self::Wasm => &[0x0, 0x61, 0x73, 0x6d],
            _ => &[],
        }
    }
}
