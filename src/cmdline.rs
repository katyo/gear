/*!
Command-line arguments and command processing
 */

use gear::system::{Path, PathBuf};
use std::str::FromStr;
use structopt::StructOpt;

/// Default rules files
const RULES_FILES: &str = "Gearfile, Gearfile.js, Gear.js";

/// Default config files
const CONFIG_FILES: &str = "gear.json, gear.yaml, gear.toml";

#[cfg(unix)]
const PATHS_DELIMITER: &str = ":";

#[cfg(windows)]
const PATHS_DELIMITER: &str = ";";

/// Flexible build tool
#[derive(StructOpt, Debug)]
pub struct Args {
    /// Rules file
    #[structopt(
        short = "f",
        long = "file",
        env = "GEAR_FILE",
        default_value = RULES_FILES
    )]
    pub file: PathBuf,

    /// Config file
    #[structopt(
        short = "i",
        long = "config",
        env = "GEAR_CONFIG",
        default_value = CONFIG_FILES
    )]
    pub config: PathBuf,

    /// ES6 modules paths
    #[structopt(
        short = "I",
        long = "path",
        alias = "include-dir",
        env = "GEAR_PATH",
        value_delimiter = PATHS_DELIMITER,
        require_delimiter = true
    )]
    pub paths: Vec<PathBuf>,

    /// Base source directory
    #[structopt(
        short = "C",
        long = "base",
        alias = "directory",
        env = "GEAR_BASE",
        default_value = ""
    )]
    pub base: PathBuf,

    /// Destination directory
    #[structopt(
        short = "O",
        long = "dest",
        env = "GEAR_DEST",
        default_value = "target"
    )]
    pub dest: PathBuf,

    /// Logging filter
    ///
    /// Set log level or filter in form [<topic>=]<level> where <level> is one of trace, debug, info, warn or error.
    /// You can set an optional <topic> prefix to filter logs by crate.
    #[structopt(
        short = "l",
        long = "log",
        env = "GEAR_LOG",
        default_value = "info",
        require_delimiter = true
    )]
    pub log: Vec<String>,

    /// Generate completions
    #[structopt(name = "shell", long = "completions", possible_values = &structopt::clap::Shell::variants())]
    pub completions: Option<structopt::clap::Shell>,

    /// Number of jobs nurs simultaneously
    #[structopt(name = "jobs", short = "j", long = "jobs")]
    pub jobs: Option<usize>,

    /// Print database
    ///
    /// Prints known goals and variables.
    /// You can use pattern to filter printed data.
    #[structopt(
        name = "format",
        short = "p",
        long = "print-db",
        alias = "print-data-base",
        possible_values = PRINT_VALUES,
    )]
    pub print_db: Option<Option<Print>>,

    /// Do not invoke rules
    ///
    /// Check consistency only
    #[structopt(short = "n", long = "dry-run")]
    pub dry_run: bool,

    /// Watch mode
    ///
    /// In this mode goals will be updated when updating dependencies.
    #[cfg(feature = "watch")]
    #[structopt(short = "w", long = "watch")]
    pub watch: bool,

    /// WebUI URL
    ///
    /// Start HTTP API and Web-based UI under this URL.
    /// Both TCP and Unix Domain sockets supported.
    #[cfg(feature = "webui")]
    #[structopt(short = "b", long = "webui")]
    pub webui: Option<tide::http::Url>,

    /// Targets and variables
    ///
    /// You can pass goals to build via command line as `goal1 goal2 ...`.
    /// You can set variables via `Gear.{toml,yaml,json}` file or via command line as `name1=value1 name2=value2 ...`. Variables passed via command line overrides variables passed via config.
    ///
    /// Use `-p` flag to print available goals and variables.
    #[structopt()]
    pub input: Vec<Input>,
}

impl Args {
    pub fn get_base(&self) -> String {
        self.base.display().to_string()
    }

    pub fn get_dest(&self) -> String {
        self.dest.display().to_string()
    }

    pub async fn find_file(&self) -> Option<String> {
        self.file_select(&self.file, RULES_FILES).await
    }

    pub async fn find_config(&self) -> Option<String> {
        self.file_select(&self.config, CONFIG_FILES).await
    }

    pub fn gen_completions(&self) {
        if let Some(shell) = self.completions {
            Self::clap().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut std::io::stdout());
        }
    }

    pub fn get_jobs(&self) -> usize {
        self.jobs.unwrap_or_else(|| num_cpus::get())
    }

    pub fn get_print(&self) -> Option<Print> {
        self.print_db.map(|print| print.unwrap_or_default())
    }

    pub fn get_log(&self) -> String {
        self.log.join(",")
    }

    pub fn get_paths<'i>(&'i self) -> impl Iterator<Item = String> + 'i {
        self.paths.iter().map(|path| path.display().to_string())
    }

    pub fn get_vars<'i>(&'i self) -> impl Iterator<Item = (String, String)> + 'i {
        self.input.iter().filter_map(|item| item.to_pair())
    }

    pub fn get_goals<'i>(&'i self) -> impl Iterator<Item = String> + 'i {
        self.input.iter().filter_map(|item| item.to_name())
    }

    async fn file_select(&self, path: &Path, candidates: &str) -> Option<String> {
        if path != Path::new(candidates) {
            return path.to_str().map(String::from);
        }
        for candidate in candidates.split(", ") {
            let path = self.base.join(candidate);
            if path.is_file().await {
                return Some(candidate.to_string());
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Print {
    Goals,
    Graph,
}

const PRINT_VALUES: &[&str] = &["plain", "dot"];

impl FromStr for Print {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "graph" | "graphviz" | "dot" => Self::Graph,
            _ => Self::Goals,
        })
    }
}

impl Default for Print {
    fn default() -> Self {
        Self::Goals
    }
}

#[derive(Clone, Debug)]
pub enum Input {
    Pair(String, String),
    Name(String),
}

impl FromStr for Input {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(p) = s.find('=') {
            let key = (&s[0..p]).into();
            let val = (&s[p + 1..]).into();
            Self::Pair(key, val)
        } else {
            Self::Name(s.into())
        })
    }
}

impl Input {
    pub fn to_name(&self) -> Option<String> {
        if let Self::Name(name) = self {
            Some(name.clone())
        } else {
            None
        }
    }

    pub fn to_pair(&self) -> Option<(String, String)> {
        if let Self::Pair(key, val) = self {
            Some((key.clone(), val.clone()))
        } else {
            None
        }
    }
}
