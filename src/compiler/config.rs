use crate::{
    qjs,
    system::{check_access, which, AccessMode, Path},
    Map, Result, Set,
};
use std::{
    borrow::Cow,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
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
    fn fmt_args(&self, out: &mut Vec<String>);
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

pub type StrOpt = Option<String>;
pub type StrList = Vec<String>;

pub type OptSet = Set<OptVal>;
pub type OptMap = Map<String, OptVal>;

pub type StrSet = Set<String>;
pub type StrMap = Map<String, String>;

impl FormatArgs for (&str, &OptSet) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        let set = self.1;
        for val in set {
            use OptVal::*;
            match val {
                Bool(val) if *val => {
                    out.push(format!("{}", opt));
                }
                Int(val) => {
                    out.push(format!("{}{}", opt, val));
                }
                Str(val) => {
                    out.push(format!("{}{}", opt, val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &OptMap) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        let map = self.1;
        for (name, val) in map {
            use OptVal::*;
            match val {
                Bool(val) => {
                    out.push(format!(
                        "{}{}{}",
                        opt,
                        if *val { "" } else { "no-" },
                        OptName(name)
                    ));
                }
                Int(val) => {
                    out.push(format!("{}{}={}", opt, OptName(name), val));
                }
                Str(val) => {
                    out.push(format!("{}{}={}", opt, OptName(name), val));
                }
                _ => {}
            }
        }
    }
}

impl FormatArgs for (&str, &StrOpt) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        if let Some(val) = self.1 {
            out.push(format!("{}{}", opt, val));
        }
    }
}

impl FormatArgs for (&str, &StrList) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        let list = self.1;
        for val in list {
            out.push(format!("{}{}", opt, val));
        }
    }
}

impl FormatArgs for StrList {
    fn fmt_args(&self, out: &mut Vec<String>) {
        out.extend(self.clone());
    }
}

impl FormatArgs for (&str, &StrSet) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        let set = self.1;
        for val in set {
            out.push(format!("{}{}", opt, val));
        }
    }
}

impl FormatArgs for (&str, &StrMap) {
    fn fmt_args(&self, out: &mut Vec<String>) {
        let opt = self.0;
        let map = self.1;
        for (name, val) in map {
            out.push(format!("{}{}={}", opt, name, val));
        }
    }
}

#[derive(Debug, Default, Clone, qjs::FromJs, qjs::IntoJs)]
pub struct CommonOpts {
    pub opt: StrOpt,       // -O...
    pub stdlib: StrOpt,    // -stdlib...
    pub pic: Option<bool>, // -fPIC
    #[quickjs(default)]
    pub dbg: OptMap, // -g...
    #[quickjs(default)]
    pub mach: OptMap, // -m...
    #[quickjs(default)]
    pub feat: OptMap, // -f...
    #[quickjs(default)]
    pub flags: StrList, // ...
}

impl Hash for CommonOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.opt.hash(state);
        self.stdlib.hash(state);
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

impl FormatArgs for CommonOpts {
    fn fmt_args(&self, out: &mut Vec<String>) {
        ("-O", &self.opt).fmt_args(out);
        ("-stdlib", &self.stdlib).fmt_args(out);
        if let Some(pic) = &self.pic {
            if *pic {
                out.push("-fPIC".into());
                out.push("-fpic".into());
            }
        }
        ("-g", &self.dbg).fmt_args(out);
        ("-m", &self.mach).fmt_args(out);
        ("-f", &self.feat).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl Extend<CommonOpts> for CommonOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = CommonOpts>,
    {
        for conf in iter {
            if conf.opt.is_some() {
                self.opt = conf.opt;
            }
            if conf.stdlib.is_some() {
                self.stdlib = conf.stdlib;
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
pub struct CompileOpts {
    //pub lang: StrOpt, // -x...
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

impl Hash for CompileOpts {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.lang.hash(state);
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

impl FormatArgs for CompileOpts {
    fn fmt_args(&self, out: &mut Vec<String>) {
        //("-x", &self.lang).fmt_args(out);
        ("-std=", &self.std).fmt_args(out);
        ("-W", &self.warn).fmt_args(out);
        ("-D", &self.defs).fmt_args(out);
        ("-I", &self.dirs).fmt_args(out);
        ("-i", &self.incs).fmt_args(out);
        ("-no", &self.no).fmt_args(out);
        self.flags.fmt_args(out);
    }
}

impl Extend<CompileOpts> for CompileOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = CompileOpts>,
    {
        for conf in iter {
            //if conf.lang.is_some() {
            //    self.lang = conf.lang;
            //}
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

impl FormatArgs for LinkOpts {
    fn fmt_args(&self, out: &mut Vec<String>) {
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

impl FormatArgs for DumpOpts {
    fn fmt_args(&self, out: &mut Vec<String>) {
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

impl FormatArgs for StripOpts {
    fn fmt_args(&self, out: &mut Vec<String>) {
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

#[derive(Debug, Default, Clone, Hash, qjs::FromJs, qjs::IntoJs)]
pub struct ToolchainOpts {
    #[quickjs(default)]
    pub common: CommonOpts,
    #[quickjs(default)]
    pub compile: CompileOpts,
    #[quickjs(default)]
    pub link: LinkOpts,
    #[quickjs(default)]
    pub dump: DumpOpts,
    #[quickjs(default)]
    pub strip: StripOpts,
}

impl Extend<ToolchainOpts> for ToolchainOpts {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = ToolchainOpts>,
    {
        for conf in iter {
            self.common.extend(Some(conf.common));
            self.compile.extend(Some(conf.compile));
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
                candidates.push(format!("{}-{}", self.compiler, self.target).into());
            }
            candidates.push(self.compiler.as_str().into());
        }

        let mut this = self.clone();

        for candidate in &candidates {
            let name = candidate.as_ref();
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
