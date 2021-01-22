use crate::{qjs, Map, Set};
use std::{fmt, str};

#[derive(Debug, Clone, qjs::FromJs, qjs::IntoJs)]
#[quickjs(untagged)]
pub enum Arch {
    /// Unknown architecture
    #[quickjs(rename = "unknown")]
    Unknown,

    /// AArch64 (little endian): aarch64
    #[quickjs(rename = "aarch64")]
    Aarch64,
    /// AArch64 (big endian): aarch64_be
    #[quickjs(rename = "aarch64_be")]
    Aarch64be,
    /// AArch64 (little endian) ILP32: aarch64_32
    #[quickjs(rename = "aarch64_32")]
    Aarch64_32,
    /// Alpha arch
    #[quickjs(rename = "alpha")]
    Alpha,
    /// ARM (little endian): arm, armv.*, xscale
    #[quickjs(rename = "arm")]
    Arm,
    /// ARM (big endian): armeb
    #[quickjs(rename = "armeb")]
    Armeb,
    /// ARC: Synopsys ARC
    #[quickjs(rename = "arc")]
    Arc,
    /// AVR: Atmel AVR microcontroller
    #[quickjs(rename = "avr")]
    Avr,
    /// eBPF or extended BPF or 64-bit BPF (little endian)
    #[quickjs(rename = "bpfel")]
    Bpfel,
    /// eBPF or extended BPF or 64-bit BPF (big endian)
    #[quickjs(rename = "bpfeb")]
    Bpfeb,
    /// CSKY: csky
    #[quickjs(rename = "csky")]
    Csky,
    /// Hexagon: hexagon
    #[quickjs(rename = "hexagon")]
    Hexagon,
    /// MIPS: mips, mipsallegrex, mipsr6
    #[quickjs(rename = "mips")]
    Mips,
    /// MIPSEL: mipsel, mipsallegrexe, mipsr6el
    #[quickjs(rename = "mipsel")]
    Mipsel,
    /// MIPS64: mips64, mips64r6, mipsn32, mipsn32r6
    #[quickjs(rename = "mips64")]
    Mips64,
    /// MIPS64EL: mips64el, mips64r6el, mipsn32el, mipsn32r6el
    #[quickjs(rename = "mips64el")]
    Mips64el,
    /// MSP430: msp430
    #[quickjs(rename = "msp430")]
    Msp430,
    /// PPC: powerpc
    #[quickjs(rename = "ppc")]
    Ppc,
    /// PPC64: powerpc64, ppu
    #[quickjs(rename = "ppc64")]
    Ppc64,
    /// PPC64LE: powerpc64le
    #[quickjs(rename = "ppc64le")]
    Ppc64le,
    /// R600: AMD GPUs HD2XXX - HD6XXX
    #[quickjs(rename = "r600")]
    R600,
    /// AMDGCN: AMD GCN GPUs
    #[quickjs(rename = "amdgcn")]
    Amdgcn,
    /// RISC-V (32-bit): riscv32
    #[quickjs(rename = "riscv32")]
    Riscv32,
    /// RISC-V (64-bit): riscv64
    #[quickjs(rename = "riscv64")]
    Riscv64,
    /// Sparc: sparc
    #[quickjs(rename = "sparc")]
    Sparc,
    /// Sparcv9: Sparcv9
    #[quickjs(rename = "sparcv9")]
    Sparcv9,
    /// Sparc: (endianness = little). NB: 'Sparcle' is a CPU variant
    #[quickjs(rename = "sparcel")]
    Sparcel,
    /// SystemZ: s390x
    #[quickjs(rename = "systemz")]
    Systemz,
    /// TCE (http://tce.cs.tut.fi/): tce
    #[quickjs(rename = "tce")]
    Tce,
    /// TCE little endian (http://tce.cs.tut.fi/): tcele
    #[quickjs(rename = "tcele")]
    Tcele,
    /// Thumb (little endian): thumb, thumbv.*
    #[quickjs(rename = "thumb")]
    Thumb,
    /// Thumb (big endian): thumbeb
    #[quickjs(rename = "thumbeb")]
    Thumbeb,
    /// X86: i[3-9]86
    #[quickjs(rename = "x86")]
    X86,
    /// X86-64: amd64, x86_64
    #[quickjs(rename = "x86_64")]
    X86_64,
    /// XCore: xcore
    #[quickjs(rename = "xcore")]
    Xcore,
    /// NVPTX: 32-bit
    #[quickjs(rename = "nvptx")]
    Nvptx,
    /// NVPTX: 64-bit
    #[quickjs(rename = "nvptx64")]
    Nvptx64,
    /// le32: generic little-endian 32-bit CPU (PNaCl)
    #[quickjs(rename = "le32")]
    Le32,
    /// le64: generic little-endian 64-bit CPU (PNaCl)
    #[quickjs(rename = "le64")]
    Le64,
    /// AMDIL
    #[quickjs(rename = "amdil")]
    Amdil,
    /// AMDIL with 64-bit pointers
    #[quickjs(rename = "amdil64")]
    Amdil64,
    /// AMD HSAIL
    #[quickjs(rename = "hsail")]
    Hsail,
    /// AMD HSAIL with 64-bit pointers
    #[quickjs(rename = "hsail64")]
    Hsail64,
    /// SPIR: standard portable IR for OpenCL 32-bit version
    #[quickjs(rename = "spir")]
    Spir,
    /// SPIR: standard portable IR for OpenCL 64-bit version
    #[quickjs(rename = "spir64")]
    Spir64,
    /// Kalimba: generic kalimba
    #[quickjs(rename = "kalimba")]
    Kalimba,
    /// SHAVE: Movidius vector VLIW processors
    #[quickjs(rename = "shave")]
    Shave,
    /// Lanai: Lanai 32-bit
    #[quickjs(rename = "lanai")]
    Lanai,
    /// WebAssembly with 32-bit pointers
    #[quickjs(rename = "wasm32")]
    Wasm32,
    /// WebAssembly with 64-bit pointers
    #[quickjs(rename = "wasm64")]
    Wasm64,
    /// 32-bit RenderScript
    #[quickjs(rename = "renderscript32")]
    Renderscript32,
    /// 64-bit RenderScript
    #[quickjs(rename = "renderscript64")]
    Renderscript64,
    /// NEC SX-Aurora Vector Engine
    #[quickjs(rename = "ve")]
    Ve,
    /// Xtensa architecture
    #[quickjs(rename = "xtensa")]
    Xtensa,
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct Triple {
    pub arch: String,
    pub sub: Option<String>,
    pub vendor: String,
    pub os: String,
    pub env: Option<String>,
    pub format: Option<String>,
}

/*impl str::FromStr for Triple {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {}
}*/
