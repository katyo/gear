use crate::{
    qjs,
    system::{check_access, which, AccessMode, Path},
    Error, Map, Result, Set,
};
use std::{
    borrow::Cow,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    result::Result as StdResult,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
#[quickjs(untagged)]
pub enum OptVal {
    Off,
    Bool(bool),
    Int(i32),
    Str(String),
}

impl Default for OptVal {
    fn default() -> Self {
        Self::Off
    }
}

pub trait FormatArgs {
    fn fmt_args(self, out: &mut Vec<String>);
}

pub struct OptName<T>(pub T);

impl<T: AsRef<str>> Display for OptName<T> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let name = self.0.as_ref();
        let mut pieces = name.split('_');
        if let Some(piece) = pieces.next() {
            piece.fmt(f)?;
            for piece in pieces {
                '-'.fmt(f)?;
                piece.fmt(f)?;
            }
        }
        Ok(())
    }
}

pub type BoolOpt = Option<bool>;
pub type StrOpt = Option<String>;
pub type ValOpt = Option<OptVal>;
pub type StrList = Vec<String>;

pub type OptSet = Set<OptVal>;
pub type OptMap = Map<String, OptVal>;

pub type StrSet = Set<String>;
pub type StrMap = Map<String, String>;

impl FormatArgs for (&str, &BoolOpt) {
    fn fmt_args(self, out: &mut Vec<String>) {
        if let Some(true) = self.1 {
            out.push(self.0.into());
        }
    }
}

impl FormatArgs for (&str, &ValOpt) {
    fn fmt_args(self, out: &mut Vec<String>) {
        if let Some(val) = self.1 {
            (self.0, val).fmt_args(out);
        }
    }
}

impl FormatArgs for (&str, &OptVal) {
    fn fmt_args(self, out: &mut Vec<String>) {
        match self.1 {
            OptVal::Bool(true) => out.push(self.0.into()),
            OptVal::Int(val) => out.push(format!("{}{}", self.0, val)),
            OptVal::Str(val) => out.push(format!("{}{}", self.0, val)),
            _ => (),
        }
    }
}

impl FormatArgs for (&str, &OptSet) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for val in self.1 {
            use OptVal::*;
            match val {
                Bool(val) if *val => {
                    out.push(format!("{}", self.0));
                }
                Int(val) => {
                    out.push(format!("{}{}", self.0, val));
                }
                Str(val) => {
                    out.push(format!("{}{}", self.0, val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &str, &OptSet) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for val in self.2 {
            use OptVal::*;
            match val {
                Bool(val) if *val => {
                    out.push(format!("{}", self.0));
                }
                Int(val) => {
                    out.push(format!("{}{}{}", self.0, self.1, val));
                }
                Str(val) => {
                    out.push(format!("{}{}{}", self.0, self.1, val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &str, &OptMap) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for (name, val) in self.2 {
            use OptVal::*;
            match val {
                Bool(val) => {
                    out.push(format!(
                        "{}{}",
                        if *val { self.0 } else { self.1 },
                        OptName(name)
                    ));
                }
                Int(val) => {
                    out.push(format!("{}{}={}", self.0, OptName(name), val));
                }
                Str(val) => {
                    out.push(format!("{}{}={}", self.0, OptName(name), val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &OptMap) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for (name, val) in self.1 {
            use OptVal::*;
            match val {
                Bool(val) => {
                    out.push(format!(
                        "{}{}{}",
                        self.0,
                        if *val { "" } else { "no-" },
                        OptName(name)
                    ));
                }
                Int(val) => {
                    out.push(format!("{}{}={}", self.0, OptName(name), val));
                }
                Str(val) => {
                    out.push(format!("{}{}={}", self.0, OptName(name), val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &StrOpt) {
    fn fmt_args(self, out: &mut Vec<String>) {
        if let Some(val) = self.1 {
            out.push(format!("{}{}", self.0, val));
        }
    }
}

impl FormatArgs for (&str, &StrList) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for val in self.1 {
            out.push(format!("{}{}", self.0, val));
        }
    }
}

impl FormatArgs for &StrList {
    fn fmt_args(self, out: &mut Vec<String>) {
        out.extend(self.clone());
    }
}

impl FormatArgs for (&str, &StrSet) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for val in self.1 {
            out.push(format!("{}{}", self.0, val));
        }
    }
}

impl FormatArgs for (&str, &StrMap) {
    fn fmt_args(self, out: &mut Vec<String>) {
        for (name, val) in self.1 {
            out.push(format!("{}{}={}", self.0, name, val));
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct BaseOpts {
    pub stdlib: StrOpt,  // -stdlib...
    pub sysroot: StrOpt, // --sysroot ...
    pub pic: BoolOpt,    // -fPIC
    pub opt: ValOpt,     // -O...
    #[quickjs(default)]
    pub dbg: OptMap, // -g...
    #[quickjs(default)]
    pub mach: OptMap, // -m...
    #[quickjs(default)]
    pub feat: OptMap, // -f...
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for BaseOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.opt.hash(state);
        self.stdlib.hash(state);
        self.sysroot.hash(state);
        self.pic.hash(state);
        for (key, val) in &self.dbg {
            key.hash(state);
            val.hash(state);
        }
        for (key, val) in &self.mach {
            key.hash(state);
            val.hash(state);
        }
        for (key, val) in &self.feat {
            key.hash(state);
            val.hash(state);
        }
        self.flags.hash(state);
    }
}

impl FormatArgs for &BaseOpts {
    fn fmt_args(self, out: &mut Vec<String>) {
        ("-O", &self.opt).fmt_args(out);
        ("-stdlib", &self.stdlib).fmt_args(out);
        ("--sysroot=", &self.sysroot).fmt_args(out);
        if let Some(true) = self.pic {
            out.push("-fPIC".into());
            out.push("-fpic".into());
        }
        ("-g", &self.dbg).fmt_args(out);
        ("-m", &self.mach).fmt_args(out);
        ("-f", &self.feat).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl Extend<BaseOpts> for BaseOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = BaseOpts>,
    {
        for conf in iter {
            if conf.opt.is_some() {
                self.opt = conf.opt;
            }
            if conf.stdlib.is_some() {
                self.stdlib = conf.stdlib;
            }
            if conf.sysroot.is_some() {
                self.sysroot = conf.sysroot;
            }
            if conf.pic.is_some() {
                self.pic = conf.pic;
            }
            self.dbg.extend(conf.dbg);
            self.mach.extend(conf.mach);
            self.feat.extend(conf.feat);
            self.flags.extend(conf.flags);
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct CCompileOpts {
    pub std: StrOpt, // -std...
    #[quickjs(default)]
    pub warn: OptMap, // -W...
    #[quickjs(default)]
    pub defs: StrMap, // -D...
    #[quickjs(default)]
    pub dirs: StrSet, // -I...
    #[quickjs(default)]
    pub incs: StrSet, // -i...
    #[quickjs(default)]
    pub no: StrSet, // -no... stdinc,stdinc++
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for CCompileOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.std.hash(state);
        for (key, val) in &self.warn {
            key.hash(state);
            val.hash(state);
        }
        for (key, val) in &self.defs {
            key.hash(state);
            val.hash(state);
        }
        for val in &self.dirs {
            val.hash(state);
        }
        for val in &self.incs {
            val.hash(state);
        }
        for val in &self.no {
            val.hash(state);
        }
        self.flags.hash(state);
    }
}

impl FormatArgs for &CCompileOpts {
    fn fmt_args(self, out: &mut Vec<String>) {
        ("-std=", &self.std).fmt_args(out);
        ("-W", &self.warn).fmt_args(out);
        ("-D", &self.defs).fmt_args(out);
        ("-I", &self.dirs).fmt_args(out);
        ("-i", &self.incs).fmt_args(out);
        ("-no", &self.no).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl FormatArgs for (&BaseOpts, &CCompileOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        self.0.fmt_args(out);
        self.1.fmt_args(out);
    }
}

impl FormatArgs for (&BaseOpts, &CCompileOpts, &CCompileOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        self.0.fmt_args(out);
        self.1.fmt_args(out);
        self.2.fmt_args(out);
    }
}

impl Extend<CCompileOpts> for CCompileOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = CCompileOpts>,
    {
        for conf in iter {
            if conf.std.is_some() {
                self.std = conf.std;
            }
            self.warn.extend(conf.warn);
            self.defs.extend(conf.defs);
            self.dirs.extend(conf.dirs);
            self.incs.extend(conf.incs);
            self.no.extend(conf.no);
            self.flags.extend(conf.flags);
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct DCompileOpts {
    #[quickjs(default)]
    pub release: BoolOpt, // --release
    pub betterc: BoolOpt, // --betterC
    pub check: OptMap, // bounds (--boundscheck=off,on,safeonly) printf (--check-printf-calls) action (--checkaction=D,C,halt,context)
    #[quickjs(default)]
    pub debug: OptMap, // -d-debug=...
    #[quickjs(default)]
    pub version: OptMap, // -d-version=...
    #[quickjs(default)]
    pub preview: StrMap, // --preview=...
    #[quickjs(default)]
    pub feat: OptMap, // --enable-... --disable-... -f... switch-errors,pre/postconditions,transition=...
    #[quickjs(default)]
    pub dirs: StrSet, // -I=...
    #[quickjs(default)]
    pub no: StrSet, // --no... asm,gc
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for DCompileOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.release.hash(state);
        self.betterc.hash(state);
        for (key, val) in &self.check {
            key.hash(state);
            val.hash(state);
        }
        for val in &self.debug {
            val.hash(state);
        }
        for val in &self.version {
            val.hash(state);
        }
        for (key, val) in &self.feat {
            key.hash(state);
            val.hash(state);
        }
        for val in &self.dirs {
            val.hash(state);
        }
        for val in &self.no {
            val.hash(state);
        }
        self.flags.hash(state);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DCompilerKind {
    //Dmd,
    Gdc,
    Ldc,
}

impl From<CompilerKind> for DCompilerKind {
    fn from(kind: CompilerKind) -> Self {
        match kind {
            CompilerKind::Gcc => Self::Gdc,
            CompilerKind::Llvm => Self::Ldc,
        }
    }
}

impl FormatArgs for (DCompilerKind, &DCompileOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        match self.0 {
            DCompilerKind::Gdc => {
                ("-frelease", &self.1.release).fmt_args(out);
                if let Some(false) = self.1.release {
                    out.push("-fassert".into());
                }
                if let Some(true) = self.1.betterc {
                    out.push("-fno-druntime".into());
                    /*out.push("-fno-rtti".into());
                    out.push("-fno-exceptions".into());
                    out.push("-fno-moduleinfo".into());*/
                }
                if let Some(OptVal::Str(check)) = self.1.check.get("bounds") {
                    out.push(format!("-fbounds-check={}", check));
                }
                ("-fdebug", "=", &self.1.debug).fmt_args(out);
                ("-fversion", &self.1.version).fmt_args(out);
                ("-ftransition=", &self.1.preview).fmt_args(out);
                ("-f", &self.1.feat).fmt_args(out);
                ("-I", &self.1.dirs).fmt_args(out);
                ("-no", &self.1.no).fmt_args(out);
            }
            DCompilerKind::Ldc => {
                ("--release", &self.1.release).fmt_args(out);
                ("--betterC", &self.1.betterc).fmt_args(out);
                if let Some(OptVal::Str(check)) = self.1.check.get("bounds") {
                    out.push(format!("--boundscheck={}", check));
                }
                if let Some(OptVal::Bool(true)) = self.1.check.get("printf") {
                    out.push("--check-printf-calls".into());
                }
                if let Some(OptVal::Str(action)) = self.1.check.get("action") {
                    out.push(format!("--checkaction={}", action));
                }
                ("--d-debug", "=", &self.1.debug).fmt_args(out);
                ("--d-version=", &self.1.version).fmt_args(out);
                ("--preview=", &self.1.preview).fmt_args(out);
                ("--enable-", "--disable-", &self.1.feat).fmt_args(out);
                ("-I=", &self.1.dirs).fmt_args(out);
                ("-J=", &self.1.dirs).fmt_args(out);
                ("-no", &self.1.no).fmt_args(out);
                self.1.flags.fmt_args(out);
            }
        }
    }
}

impl FormatArgs for (DCompilerKind, &BaseOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        match self.0 {
            DCompilerKind::Gdc => {
                ("-O", &self.1.opt).fmt_args(out);
                if let Some(true) = self.1.pic {
                    out.push("-fPIC".into());
                    out.push("-fpic".into());
                }
                ("-g", &self.1.dbg).fmt_args(out);
                ("-m", &self.1.mach).fmt_args(out);
                ("-f", &self.1.feat).fmt_args(out);
            }
            DCompilerKind::Ldc => {
                ("-O", &self.1.opt).fmt_args(out);
                if let Some(lto) = self.1.feat.get("lto") {
                    match lto {
                        OptVal::Bool(true) => out.push("--flto=thin".into()),
                        OptVal::Str(lto) => out.push(format!("--flto={}", lto)),
                        _ => (),
                    }
                }
                if !self.1.dbg.is_empty() {
                    out.push("--gc".into());
                }
                for bits in &["32", "64"] {
                    if let Some(OptVal::Bool(true)) = self.1.mach.get(*bits) {
                        out.push(format!("--m{}", bits));
                    }
                }
                for key in &["arch", "cpu", "attr"] {
                    if let Some(OptVal::Str(val)) = self.1.mach.get(*key) {
                        out.push(format!("--m{}={}", key, val));
                    }
                }
                if let Some(abi) = self.1.mach.get("float_abi") {
                    match abi {
                        OptVal::Bool(true) => out.push("--float-abi=default".into()),
                        OptVal::Str(abi) => out.push(format!("--float-abi={}", abi)),
                        _ => (),
                    }
                }
            }
        }
    }
}

impl FormatArgs for (DCompilerKind, &BaseOpts, &DCompileOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        (self.0, self.1).fmt_args(out);
        (self.0, self.2).fmt_args(out);
    }
}

impl FormatArgs for (CompilerKind, &BaseOpts, &DCompileOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        let kind = DCompilerKind::from(self.0);
        (kind, self.1).fmt_args(out);
        (kind, self.2).fmt_args(out);
    }
}

impl Extend<DCompileOpts> for DCompileOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = DCompileOpts>,
    {
        for conf in iter {
            if conf.release.is_some() {
                self.release = conf.release;
            }
            if conf.betterc.is_some() {
                self.betterc = conf.betterc;
            }
            self.check.extend(conf.check);
            self.debug.extend(conf.debug);
            self.version.extend(conf.version);
            self.feat.extend(conf.feat);
            self.dirs.extend(conf.dirs);
            self.no.extend(conf.no);
            self.flags.extend(conf.flags);
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct LinkOpts {
    #[quickjs(default)]
    pub dirs: StrSet, // -L...
    #[quickjs(default)]
    pub libs: StrSet, // -l...
    #[quickjs(default)]
    pub whole_libs: StrSet, // -Wl,--whole-archive -l...
    #[quickjs(default)]
    pub no: StrSet, // -no... startfiles,defaultlibs,libc,stdlib
    pub pie: Option<OptVal>,    // -pie|-no-pie|-static-pie
    pub shared: Option<OptVal>, // -shared
    #[quickjs(default)]
    pub shareds: StrSet, // -shared-...
    pub static_: Option<OptVal>, // -static
    #[quickjs(default, rename = "static")]
    pub statics: OptSet, // -static-... pie,libgcc,libstdc++,...
    #[quickjs(default)]
    pub opts: StrList, // -Wl,...
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for LinkOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for val in &self.dirs {
            val.hash(state);
        }
        for val in &self.libs {
            val.hash(state);
        }
        for val in &self.whole_libs {
            val.hash(state);
        }
        for val in &self.no {
            val.hash(state);
        }
        self.pie.hash(state);
        self.shared.hash(state);
        for val in &self.shareds {
            val.hash(state);
        }
        self.static_.hash(state);
        for val in &self.statics {
            val.hash(state);
        }
        self.opts.hash(state);
        self.flags.hash(state);
    }
}

impl FormatArgs for &LinkOpts {
    fn fmt_args(self, out: &mut Vec<String>) {
        ("-L", &self.dirs).fmt_args(out);
        ("-l", &self.libs).fmt_args(out);
        if !self.whole_libs.is_empty() {
            out.push("-Wl,--whole-archive".into());
            ("-l", &self.libs).fmt_args(out);
            out.push("-Wl,--no-whole-archive".into());
        }
        ("-no", &self.no).fmt_args(out);
        match &self.pie {
            Some(OptVal::Bool(value)) => {
                out.push(format!("{}-pie", if *value { "" } else { "-no" }))
            }
            Some(OptVal::Str(value)) => out.push(format!("-{}-pie", value)),
            _ => {}
        }
        match &self.shared {
            Some(OptVal::Bool(value)) if *value => {
                out.push("-shared".into());
            }
            _ => {}
        }
        ("-shared-", &self.shareds).fmt_args(out);
        match &self.static_ {
            Some(OptVal::Bool(value)) if *value => {
                out.push("-static".into());
            }
            _ => {}
        }
        ("-static-", &self.statics).fmt_args(out);
        ("-Wl,", &self.opts).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl FormatArgs for (&BaseOpts, &LinkOpts) {
    fn fmt_args(self, out: &mut Vec<String>) {
        self.0.fmt_args(out);
        self.1.fmt_args(out);
    }
}

impl Extend<LinkOpts> for LinkOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = LinkOpts>,
    {
        for conf in iter {
            self.dirs.extend(conf.dirs);
            self.libs.extend(conf.libs);
            self.whole_libs.extend(conf.whole_libs);
            if conf.pie.is_some() {
                self.pie = conf.pie.clone();
            }
            if conf.shared.is_some() {
                self.shared = conf.shared.clone();
            }
            if conf.static_.is_some() {
                self.static_ = conf.static_.clone();
            }
            self.no.extend(conf.no);
            self.opts.extend(conf.opts);
            self.flags.extend(conf.flags);
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct DumpOpts {
    pub target: StrOpt, // -b...
    pub arch: StrOpt,   // -m...
    #[quickjs(default)]
    pub disasm: OptSet, // -M...
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for DumpOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.target.hash(state);
        self.arch.hash(state);
        for val in &self.disasm {
            val.hash(state);
        }
        self.flags.hash(state);
    }
}

impl FormatArgs for &DumpOpts {
    fn fmt_args(self, out: &mut Vec<String>) {
        ("-b", &self.target).fmt_args(out);
        ("-m", &self.arch).fmt_args(out);
        ("-M", &self.disasm).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl Extend<DumpOpts> for DumpOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = DumpOpts>,
    {
        for conf in iter {
            if conf.target.is_some() {
                self.target = conf.target;
            }
            if conf.arch.is_some() {
                self.arch = conf.arch;
            }
            self.disasm.extend(conf.disasm);
        }
    }
}

#[derive(Debug, Clone, Default, qjs::FromJs, qjs::IntoJs)]
pub struct StripOpts {
    /// Strip options `--strip-...`
    /// (all, debug, dwo, unneeded)
    #[quickjs(default)]
    strip: StrSet,
    /// Keep options `--strip-...`
    /// (file-symbols)
    #[quickjs(default)]
    keep: StrSet,
    /// Discard options `--discard-...`
    /// (all, local)
    #[quickjs(default)]
    discard: StrSet,
    /// Remove or keep specified symbols
    #[quickjs(default)]
    symbols: Map<String, Option<bool>>,
    /// Other flags
    #[quickjs(default)]
    flags: StrList,
}

impl Hash for StripOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for val in &self.strip {
            val.hash(state);
        }
        for val in &self.keep {
            val.hash(state);
        }
        for val in &self.discard {
            val.hash(state);
        }
        for val in &self.symbols {
            val.hash(state);
        }
        self.flags.hash(state);
    }
}

impl FormatArgs for &StripOpts {
    fn fmt_args(self, out: &mut Vec<String>) {
        ("--strip-", &self.strip).fmt_args(out);
        ("--keep-", &self.keep).fmt_args(out);
        ("--discard-", &self.discard).fmt_args(out);
        for (symbol, option) in &self.symbols {
            if let Some(value) = option {
                out.push(format!(
                    "--{}-symbol={}",
                    if *value { "keep" } else { "strip" },
                    symbol
                ));
            }
        }
        self.flags.fmt_args(out);
    }
}

impl Extend<Self> for StripOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Self>,
    {
        for conf in iter {
            self.strip.extend(conf.strip);
            self.keep.extend(conf.keep);
            self.discard.extend(conf.discard);
            self.symbols.extend(conf.symbols);
            self.flags.extend(conf.flags);
        }
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub struct ToolchainOpts {
    pub base: BaseOpts,
    pub cc: CCompileOpts,
    pub c: CCompileOpts,
    pub cxx: CCompileOpts,
    pub d: DCompileOpts,
    pub link: LinkOpts,
    pub dump: DumpOpts,
    pub strip: StripOpts,
}

impl<'js> qjs::FromJs<'js> for ToolchainOpts {
    fn from_js(_ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let base = val.get::<BaseOpts>()?;
        let obj = val.get::<qjs::Object>()?;
        let cc = obj
            .get::<_, Option<CCompileOpts>>("cc")?
            .unwrap_or_default();
        let c = obj.get::<_, Option<CCompileOpts>>("c")?.unwrap_or_default();
        let mut cxx = obj
            .get::<_, Option<CCompileOpts>>("cxx")?
            .unwrap_or_default();
        if let Some(cxx_) = obj.get::<_, Option<CCompileOpts>>("c++")? {
            cxx.extend(Some(cxx_));
        }
        let d = obj.get::<_, Option<DCompileOpts>>("d")?.unwrap_or_default();
        let link = obj.get::<_, Option<LinkOpts>>("link")?.unwrap_or_default();
        let dump = obj.get::<_, Option<DumpOpts>>("dump")?.unwrap_or_default();
        let strip = obj
            .get::<_, Option<StripOpts>>("strip")?
            .unwrap_or_default();
        Ok(Self {
            base,
            cc,
            c,
            cxx,
            d,
            link,
            dump,
            strip,
        })
    }
}

impl<'js> qjs::IntoJs<'js> for ToolchainOpts {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        let val = self.base.into_js(ctx)?;
        if let Some(obj) = val.as_object() {
            obj.set("cc", self.cc.into_js(ctx)?)?;
            obj.set("c", self.c.into_js(ctx)?)?;
            obj.set("cxx", self.cxx.into_js(ctx)?)?;
            obj.set("d", self.d.into_js(ctx)?)?;
            obj.set("link", self.link.into_js(ctx)?)?;
            obj.set("dump", self.dump.into_js(ctx)?)?;
            obj.set("strip", self.strip.into_js(ctx)?)?;
        }
        Ok(val)
    }
}

impl Extend<ToolchainOpts> for ToolchainOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = ToolchainOpts>,
    {
        for conf in iter {
            self.base.extend(Some(conf.base));
            self.cc.extend(Some(conf.cc));
            self.c.extend(Some(conf.c));
            self.cxx.extend(Some(conf.cxx));
            self.d.extend(Some(conf.d));
            self.link.extend(Some(conf.link));
            self.dump.extend(Some(conf.dump));
            self.strip.extend(Some(conf.strip));
        }
    }
}

#[derive(Debug, Clone, Default, qjs::FromJs, qjs::IntoJs)]
pub struct DetectOpts {
    #[quickjs(default)]
    pub compiler: String,
    #[quickjs(default)]
    pub target: String,
}

impl Extend<DetectOpts> for DetectOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = DetectOpts>,
    {
        for conf in iter {
            if !conf.compiler.is_empty() {
                self.compiler = conf.compiler;
            }
            if !conf.target.is_empty() {
                self.target = conf.target;
            }
        }
    }
}

impl DetectOpts {
    pub async fn detect(&self) -> Result<Self> {
        let mut candidates = Vec::<Cow<'static, str>>::new();

        if self.compiler.is_empty() {
            if self.target.is_empty() {
                candidates.push("gcc".into());
                candidates.push("clang".into());
            } else {
                candidates.push(format!("{}-gcc", self.target).into());
                candidates.push(format!("{}-clang", self.target).into());
                candidates.push("clang".into());
            }
        } else {
            if self.target.is_empty() {
                candidates.push(self.compiler.as_str().into());
            } else {
                candidates.push(format!("{}-{}", self.target, self.compiler).into());
            }
            candidates.push(self.compiler.as_str().into());
        }

        let mut this = self.clone();

        for candidate in &candidates {
            let name = candidate.as_ref();
            log::debug!("Detect compiler: `{}`", name);
            let path = Path::new(name);
            if path.is_file().await {
                this.compiler = name.into();
                break;
            } else if let Some(path) = which(name).await {
                this.compiler = path.display().to_string();
                break;
            }
        }

        check_access(&this.compiler, AccessMode::EXECUTE).await?;

        Ok(this)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, qjs::FromJs, qjs::IntoJs)]
#[quickjs(untagged, rename_all = "lowercase")]
pub enum CompilerKind {
    Gcc,
    Llvm,
}

impl FromStr for CompilerKind {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self> {
        if name.ends_with("gcc") || name.ends_with("g++") {
            Ok(Self::Gcc)
        } else if name.ends_with("clang") || name.ends_with("clang++") {
            Ok(Self::Llvm)
        } else {
            Err(format!("Unsupported compiler: {}", name).into())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CInputKind {
    C,
    Cxx,
    D,
    Asm,
}

impl FromStr for CInputKind {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s {
            "c" => Ok(Self::C),
            "cpp" | "cxx" | "c++" => Ok(Self::Cxx),
            "d" => Ok(Self::D),
            "S" | "s" | "asm" => Ok(Self::Asm),
            _ => Err(()),
        }
    }
}

impl CInputKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Cxx => "c++",
            Self::D => "d",
            Self::Asm => "S",
        }
    }

    pub fn from_name(name: impl AsRef<str>) -> Result<Self> {
        let name = name.as_ref();
        let ext = name
            .rsplit('.')
            .next()
            .ok_or_else(|| format!("Unable to determine extension of source file `{}`", name))?;
        let kind =
            Self::from_str(ext).map_err(|_| format!("Unknown source file extension `{}`", ext))?;
        Ok(kind)
    }
}

impl<'js> qjs::FromJs<'js> for CInputKind {
    fn from_js(_ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let val: String = val.get()?;
        val.parse()
            .map_err(|_| qjs::Error::new_from_js("string", "CInputKind"))
    }
}

impl<'js> qjs::IntoJs<'js> for CInputKind {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.as_str().into_js(ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum COutputKind {
    /// C preprocessed output
    Cpp,
    /// Assembler output
    Asm,
    /// Object file output
    Obj,
    /// Intermidiate representation
    Ir,
    /// Bitcode output
    Bc,
}

impl Default for COutputKind {
    fn default() -> Self {
        Self::Obj
    }
}

impl FromStr for COutputKind {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s {
            "c" | "cpp" | "cxx" | "c++" => Ok(Self::Cpp),
            "S" | "s" | "asm" => Ok(Self::Asm),
            "o" | "obj" => Ok(Self::Obj),
            "ir" | "ll" | "llvm" | "llvm-ir" => Ok(Self::Ir),
            "bc" | "llvm-bc" => Ok(Self::Bc),
            _ => Err(()),
        }
    }
}

impl AsRef<str> for COutputKind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl COutputKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cpp => "cpp",
            Self::Asm => "s",
            Self::Obj => "o",
            Self::Ir => "ir",
            Self::Bc => "bc",
        }
    }

    pub fn make_extension<'a>(&'a self, name: &'a str) -> &'a str {
        if let Self::Cpp = self {
            name.rsplit('.').next().unwrap_or(self.as_ref())
        } else {
            self.as_ref()
        }
    }
}

impl<'js> qjs::FromJs<'js> for COutputKind {
    fn from_js(_ctx: qjs::Ctx<'js>, val: qjs::Value<'js>) -> qjs::Result<Self> {
        let val: String = val.get()?;
        val.parse()
            .map_err(|_| qjs::Error::new_from_js("string", "COutputKind"))
    }
}

impl<'js> qjs::IntoJs<'js> for COutputKind {
    fn into_js(self, ctx: qjs::Ctx<'js>) -> qjs::Result<qjs::Value<'js>> {
        self.as_str().into_js(ctx)
    }
}
