pub use async_process::{Command, Stdio};
pub use async_std::{
    path::{Path, PathBuf},
    prelude::*,
};
pub use relative_path::*;
pub use rquickjs as qjs;
pub use std::time::{Duration, SystemTime};

use crate::Result;
use std::ffi::OsStr;

pub use faccess::AccessMode;

/// Check access to path
pub async fn access(path: &Path, mode: AccessMode) -> bool {
    let path: &std::path::Path = path.into();
    faccess::PathExt::access(path, mode).is_ok()
}

/// Find executable by name in known paths
pub async fn which(name: &str) -> Option<PathBuf> {
    which::which(name).ok().map(|path| path.into())
}

pub async fn exec_out<S: AsRef<OsStr>, A: AsRef<OsStr>>(
    cmd: S,
    args: &[A],
) -> Result<(String, String)> {
    let cmd = cmd.as_ref();
    let out = Command::new(cmd)
        .args(args)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()
        .await?;
    if !out.status.success() {
        return Err(out
            .status
            .code()
            .map(|code| format!("Failed executing `{:?}`. Status: {}", cmd, code))
            .unwrap_or_else(|| format!("Failed executing `{:?}`. Killed", cmd))
            .into());
    }
    let err = String::from_utf8(out.stderr)?;
    let out = String::from_utf8(out.stdout)?;
    Ok((out, err))
}