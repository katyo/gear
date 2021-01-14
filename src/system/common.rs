pub use async_std::{
    fs::{create_dir_all, read as read_file, remove_file, write as write_file},
    path::{Path, PathBuf},
    prelude::*,
    process::{Command, Stdio},
    task::spawn_local as spawn,
};
pub use relative_path::*;
pub use rquickjs as qjs;

use crate::{Result, Time};
use std::ffi::OsStr;

pub use faccess::AccessMode;

/// Get modified time
pub async fn modified(path: &Path) -> Result<Time> {
    let meta = path.metadata().await?;
    let time = meta.modified().or_else(|_| meta.created())?;
    Ok(time)
}

/// Check access to path
///
/// TODO: Currently this function is synchronous but it is defined as async to avoid changing definition in the future.
pub async fn access(path: &Path, mode: AccessMode) -> bool {
    let path: &std::path::Path = path.into();
    faccess::PathExt::access(path, mode).is_ok()
}

/// Find executable by name in known paths
///
/// TODO: Currently this function is synchronous but it is defined as async to avoid changing definition in the future.
pub async fn which(name: &str) -> Option<PathBuf> {
    which::which(name).ok().map(|path| path.into())
}

/// Simply execute an arbitrary program to collect output.
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

/// Temporary file which will be removed when handle is dropped
pub struct TempFile {
    path: PathBuf,
    pipe: bool,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let path = self.path.clone();

        spawn(async move {
            if let Err(error) = remove_file(&path).await {
                log::error!(
                    "Unablt to remove temporary file `{}` due to {}",
                    path.display(),
                    error
                );
            }
        });
    }
}

impl TempFile {
    /// Create temporary file in specified directory
    pub async fn new(dir: &Path, pipe: bool) -> Result<Self> {
        let mut name: [u8; 15] = [0; 15];

        let path = loop {
            Self::rand_name(&mut name);
            let file = unsafe { std::str::from_utf8_unchecked(&name) };
            let path = dir.join(file);

            if !path.exists().await {
                break path;
            }
        };

        #[cfg(unix)]
        let pipe = if pipe {
            let path_str = path.as_os_str();
            if let Err(error) = nix::unistd::mkfifo(path_str, nix::sys::stat::Mode::S_IRWXU) {
                log::error!(
                    "Unable to create named pipe `{}` due to: {}",
                    path.display(),
                    error
                );
                false
            } else {
                true
            }
        } else {
            false
        };

        Ok(Self { path, pipe })
    }

    fn rand_name(name: &mut [u8; 15]) {
        use rand::prelude::*;

        static ALPHABET: &[u8; 32] = b"123456789abcdefghijklmnopqrstuvw";

        for (i, c) in b"temp_".iter().enumerate() {
            name[i] = *c;
        }

        let mut rng = rand::thread_rng();

        for i in 5..32 {
            name[i] = *ALPHABET.iter().choose(&mut rng).unwrap();
        }
    }

    /// Get path of temporary file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read contents of temporary file
    pub async fn read(&self) -> Result<Vec<u8>> {
        Ok(read_file(&self.path).await?)
    }

    /// Write contents of temporary file
    pub async fn write(&self, data: impl AsRef<[u8]>) -> Result<()> {
        Ok(write_file(&self.path, data).await?)
    }
}
