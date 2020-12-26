/*!
Command-line arguments and command processing
 */

pub(self) use std::path::{Path, PathBuf};
pub(self) use structopt::StructOpt;

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
        env = "GEAR_PATH",
        value_delimiter = PATHS_DELIMITER,
        require_delimiter = true
    )]
    pub paths: Vec<PathBuf>,

    /// Current directory
    #[structopt(
        short = "C",
        long = "dir",
        alias = "directory",
        env = "GEAR_DIR",
        default_value = "."
    )]
    pub dir: PathBuf,

    /// Logging filter
    ///
    /// Set log level or filter in form [<topic>=]<level> where <level> is one of trace, debug, info, warn or error.
    /// You can set an optional <topic> prefix to filter logs by crate.
    #[structopt(
        short = "l",
        long = "log",
        env = "GEAR_LOG",
        default_value = "warn",
        require_delimiter = true
    )]
    pub log: Vec<String>,

    /// Generate completions
    #[structopt(name = "shell",long = "completions", possible_values = &structopt::clap::Shell::variants())]
    pub completions: Option<structopt::clap::Shell>,

    /// Print database
    ///
    /// Prints known goals and variables.
    /// You can use pattern to filter printed data.
    #[structopt(short = "p", long = "print-db", alias = "print-data-base")]
    pub print_db: bool,

    /// Do not invoke rules
    ///
    /// Check consistency only
    #[structopt(short = "n", long = "dry-run")]
    pub dry_run: bool,

    /// Watch mode
    ///
    /// In this mode goals will be updated when updating dependencies.
    #[structopt(short = "w", long = "watch")]
    pub watch: bool,

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
    pub fn get_file(&self) -> Option<&PathBuf> {
        if self.file == Path::new(RULES_FILES) {
            None
        } else {
            Some(&self.file)
        }
    }

    pub async fn find_file(&self) -> Option<String> {
        if let Some(file) = self.get_file() {
            Some(file.display().to_string())
        } else {
            select_file(RULES_FILES).await
        }
    }

    pub fn get_config(&self) -> Option<&PathBuf> {
        if self.config == Path::new(CONFIG_FILES) {
            None
        } else {
            Some(&self.config)
        }
    }

    pub async fn find_config(&self) -> Option<String> {
        if let Some(file) = self.get_config() {
            Some(file.display().to_string())
        } else {
            select_file(CONFIG_FILES).await
        }
    }

    pub fn gen_completions(&self) {
        if let Some(shell) = self.completions {
            Self::clap().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut std::io::stdout());
        }
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
}

async fn select_file(candidates: &str) -> Option<String> {
    for candidate in candidates.split(", ") {
        let path = async_std::path::Path::new(candidate);
        if path.is_file().await {
            return Some(candidate.to_string());
        }
    }
    None
}

#[derive(Clone, Debug)]
pub enum Input {
    Pair(String, String),
    Name(String),
}

impl std::str::FromStr for Input {
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
