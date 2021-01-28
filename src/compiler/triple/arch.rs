enum_impl! {
    /// Architecture
    Arch {
        /// Unknown architecture
        Unknown => "unknown",

        /// AArch64 (little endian): aarch64
        AArch64(AArch64SubArch::No) => "aarch64" "arm64",
        /// AArch64 (big endian): aarch64_be
        AArch64Eb(AArch64SubArch::No) => "aarch64_be",
        /// AArch64 (little endian) ILP32: aarch64_32
        AArch64_32(AArch64SubArch::No) => "aarch64_32" "arm64_32",
        /// Alpha arch
        Alpha => "alpha",
        /// AMDGCN: AMD GCN GPUs
        AmdGcn => "amdgcn",
        /// AMDIL
        Amdil => "amdil",
        /// AMDIL with 64-bit pointers
        Amdil64 => "amdil64",
        /// ARC: Synopsys ARC
        Arc => "arc",
        /// ARM (little endian): arm, armv.*, xscale
        Arm(ArmSubArch::No) => "arm",
        /// ARM (big endian): armeb
        ArmEb(ArmSubArch::No) => "armeb",
        /// AVR: Atmel AVR 8-bit microcontroller
        Avr => "avr",
        /// AVR32: Atmel AVR 32-bit microcontroller
        Avr32 => "avr32",

        /// eBPF or extended BPF or 64-bit BPF (little endian)
        BpfEl => "bpfel" "bpf_le",
        /// eBPF or extended BPF or 64-bit BPF (big endian)
        BpfEb => "bpfeb" "bpf_be",

        /// CSKY: csky
        CSky => "csky",

        /// Hexagon: hexagon
        Hexagon => "hexagon",
        /// AMD HSAIL
        Hsail => "hsail",
        /// AMD HSAIL with 64-bit pointers
        Hsail64 => "hsail64",

        /// Kalimba: generic kalimba
        Kalimba(KalimbaSubArch::No) => "kalimba",

        /// Lanai: Lanai 32-bit
        Lanai => "lanai",
        /// le32: generic little-endian 32-bit CPU (PNaCl)
        Le32 => "le32",
        /// le64: generic little-endian 64-bit CPU (PNaCl)
        Le64 => "le64",

        /// MIPS: mips, mipsallegrex, mipsr6
        Mips(MipsSubArch::No) => "mips",
        /// MIPSEL: mipsel, mipsallegrexe, mipsr6el
        MipsEl(MipsSubArch::No) => "mipsel" "mipsallegrexe",
        /// MIPS64: mips64, mips64r6, mipsn32, mipsn32r6
        Mips64(MipsSubArch::No) => "mips64",
        /// MIPS64EL: mips64el, mips64r6el, mipsn32el, mipsn32r6el
        Mips64El(MipsSubArch::No) => "mips64el",
        /// MSP430: msp430
        Msp430 => "msp430",

        /// NVPTX: 32-bit
        Nvptx => "nvptx",
        /// NVPTX: 64-bit
        Nvptx64 => "nvptx64",

        /// PPC: powerpc
        Ppc => "powerpc" "ppc" "ppc32",
        /// PPC: powerpcle
        PpcLe => "powerpcle" "ppcle" "ppc32le",
        /// PPC64: powerpc64, ppu
        Ppc64 => "powerpc64" "ppu" "ppc64",
        /// PPC64LE: powerpc64le
        Ppc64Le => "powerpc64le" "ppc64le",

        /// R600: AMD GPUs HD2XXX - HD6XXX
        R600 => "r600",
        /// 32-bit RenderScript
        RenderScript32 => "renderscript32",
        /// 64-bit RenderScript
        RenderScript64 => "renderscript64",
        /// RISC-V (32-bit): riscv32
        RiscV32 => "riscv32",
        /// RISC-V (64-bit): riscv64
        RiscV64 => "riscv64",

        /// SHAVE: Movidius vector VLIW processors
        Shave => "shave",
        /// Sparc: sparc
        Sparc => "sparc",
        /// Sparcv9: Sparcv9
        SparcV9 => "sparcv9",
        /// Sparc: (endianness = little). NB: 'Sparcle' is a CPU variant
        SparcEl => "sparcel",
        /// SPIR: standard portable IR for OpenCL 32-bit version
        Spir => "spir",
        /// SPIR: standard portable IR for OpenCL 64-bit version
        Spir64 => "spir64",
        /// SystemZ: s390x
        SystemZ => "systemz" "s390x",

        /// TCE (http://tce.cs.tut.fi/): tce
        Tce => "tce",
        /// TCE little endian (http://tce.cs.tut.fi/): tcele
        TceLe => "tcele",
        /// Thumb (little endian): thumb, thumbv.*
        Thumb(ArmSubArch::No) => "thumb",
        /// Thumb (big endian): thumbeb
        ThumbEb(ArmSubArch::No) => "thumbeb",

        /// NEC SX-Aurora Vector Engine
        Ve => "ve",

        /// WebAssembly with 32-bit pointers
        Wasm32 => "wasm32",
        /// WebAssembly with 64-bit pointers
        Wasm64 => "wasm64",

        /// X86: i[3-6]86
        X86 => "x86" "i386" "i486" "i586" "i686"/* "i786" "i886" "i986"*/,
        /// X86-64: amd64, x86_64
        X86_64 => "x86_64" "amd64",
        /// XCore: xcore
        XCore => "xcore",
        /// Xtensa architecture
        Xtensa => "xtensa",
    } (s) {
        let arm_subarch_off: usize = if s.starts_with("xscale") || s.starts_with("iwmmxt") || s.starts_with("thumb") || s.starts_with("aarch") {
            5
        } else if s.starts_with("arm") {
            3
        } else {
            0
        };

        if arm_subarch_off > 0 {
            let big_endian = s.ends_with("eb");
            let first_char = s.as_bytes()[0];
            let subarch_s = &s[arm_subarch_off ..];

            if subarch_s.starts_with("64") {
                let sub_arch = AArch64SubArch::from_str(&subarch_s[2 ..]).unwrap();
                Ok(if big_endian { Self::AArch64Eb(sub_arch) } else { Self::AArch64(sub_arch) })
            } else {
                let sub_arch = if first_char == b'x' || first_char == b'i' {
                    ArmSubArch::V5e
                } else {
                    ArmSubArch::from_str(subarch_s).unwrap()
                };
                Ok(if first_char == b't' {
                    if big_endian {
                        Self::ThumbEb(sub_arch)
                    } else {
                        Self::Thumb(sub_arch)
                    }
                } else {
                    if big_endian {
                        Self::ArmEb(sub_arch)
                    } else {
                        Self::Arm(sub_arch)
                    }
                })
            }
        } else {
            let kalimba_subarch_off = if s.starts_with("kalimba") {
                7
            } else {
                0
            };

            if kalimba_subarch_off > 0 {
                let sub_arch = KalimbaSubArch::from_str(&s[kalimba_subarch_off ..]).unwrap();
                Ok(Self::Kalimba(sub_arch))
            } else {
                let mips_subarch_off = if s.starts_with("mips") {
                    4
                } else if s.starts_with("mipsn32") {
                    7
                } else {
                    0
                };

                if mips_subarch_off > 0 {
                    let little_endian = s.ends_with("el");
                    let subarch_s = &s[mips_subarch_off ..];

                    if subarch_s.starts_with("64") {
                        let sub_arch = MipsSubArch::from_str(&subarch_s[2 ..]).unwrap();
                        Ok(if little_endian {
                            Self::Mips64El(sub_arch)
                        } else {
                            Self::Mips64(sub_arch)
                        })
                    } else {
                        let sub_arch = MipsSubArch::from_str(subarch_s).unwrap();
                        Ok(if little_endian {
                            Self::MipsEl(sub_arch)
                        } else {
                            Self::Mips(sub_arch)
                        })
                    }
                } else {
                    Err(())
                }
            }
        }
    }

    /// ARM sub-architecture
    ArmSubArch {
        No => "",

        V2 => "v2",
        V2a => "v2a",

        V3 => "v3",
        V3m => "v3m",

        V4 => "v4",
        V4t => "v4t",

        V5 => "v5",
        V5e => "v5t" "v5e" "v5te" "v5tej",

        V6 => "v6",
        V6k => "v6k",
        V6kz => "v6kz",
        V6t2 => "v6t2",
        V6m => "v6m" "v6-m",

        V7 => "v7" "v7-a",
        V7em => "v7em" "v7e-m",
        V7m => "v7m",
        V7k => "v7k",
        V7r => "v7r" "v7-r",
        V7s => "v7s",
        V7ve => "v7ve",

        V8 => "v8",
        V8a => "v8a" "v8-a",
        V8p1a => "v8.1a" "v8.1-a",
        V8p2a => "v8.2a" "v8.2-a",
        V8p3a => "v8.3a" "v8.3-a",
        V8p4a => "v8.4a" "v8.4-a",
        V8p5a => "v8.5a" "v8.5-a",
        V8p6a => "v8.6a" "v8.6-a",
        V8r => "v8r" "v8-r",

        V8mBase => "v8m.base" "v8-m.base",
        V8mMain => "v8m.main" "v8-m.main",
        V8p1mMain => "v8.1m.main" "v8-m.main",
    }

    /// AArch64 sub-architecture
    AArch64SubArch {
        No => "",

        V8 => "v8",
        V8a => "v8a" "v8-a",
        V8p1a => "v8.1a" "v8.1-a",
        V8p2a => "v8.2a" "v8.2-a",
        V8p3a => "v8.3a" "v8.3-a",
        V8p4a => "v8.4a" "v8.4-a",
        V8p5a => "v8.5a" "v8.5-a",
        V8p6a => "v8.6a" "v8.6-a",
        V8r => "v8r" "v8-r",

        V8mBase => "v8m.base" "v8-m.base",
        V8mMain => "v8m.main" "v8-m.main",
        V8p1mMain => "v8.1m.main" "v8-m.main",

        E => "e",
    }

    /// Kalimba sub-architecture
    KalimbaSubArch {
        No => "",

        V3 => "v3",
        V4 => "v4",
        V5 => "v5",
    }

    /// Mips sub-architecture
    MipsSubArch {
        No => "",

        R6 => "r6",
    }

    /// PowerPC sub-architecture
    PpcSubArch {
        No => "",

        Spe => "spe",
    }
}
