/*!
Command-line arguments and command processing
 */

pub(self) use std::path::PathBuf;
pub(self) use structopt::StructOpt;

/// Logging levels list
const LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];

/// Flexible build tool
#[derive(StructOpt, Debug)]
pub struct Args {
    /// Rules file
    #[structopt(short, long, env, default_value = "Gearfile")]
    pub rules_file: PathBuf,

    /// Source directory
    #[structopt(short, long, env, default_value = ".")]
    pub source_dir: PathBuf,

    /// Target directory
    #[structopt(short, long, env, default_value = "target")]
    pub target_dir: PathBuf,

    /// Logging level
    #[structopt(short, long, env, default_value = "warn", possible_values = LOG_LEVELS)]
    pub log_level: String,

    /// Print database
    ///
    /// Prints known goals and variables.
    /// You can use pattern to filter printed data.
    #[structopt(short, long, env)]
    pub print_db: Option<Option<String>>,

    /// Do not run commands
    #[structopt(short, long, env)]
    pub dry_run: bool,

    /// Watch mode
    ///
    /// In this mode goals will be updated when updating dependencies.
    #[structopt(short, long, env)]
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
