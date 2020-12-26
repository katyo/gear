mod common;
pub use common::*;

use crate::{qjs, Map};

#[derive(qjs::FromJs)]
pub struct ExecArg {
    pub cmd: String,
    #[quickjs(default)]
    pub args: Option<Vec<String>>,
    #[quickjs(default)]
    pub envs: Option<Map<String, String>>,
    #[quickjs(default)]
    pub cwd: Option<String>,
    #[quickjs(default)]
    pub input: Option<String>,
}

#[derive(qjs::IntoJs)]
pub struct ExecRes {
    pub status: Option<i32>,
    pub output: String,
    pub error: String,
}

#[qjs::bind(module, public)]
#[quickjs(bare)]
mod js {
    use super::*;

    pub async fn sleep(msec: u64) {
        async_std::task::sleep(std::time::Duration::from_millis(msec)).await;
    }

    pub async fn is_file(path: String) -> bool {
        let path = Path::new(&path);
        path.is_file().await
    }

    pub async fn is_dir(path: String) -> bool {
        let path = Path::new(&path);
        path.is_dir().await
    }

    pub async fn exec(input: ExecArg) -> qjs::Result<ExecRes> {
        let mut cmd = Command::new(input.cmd);
        if let Some(args) = input.args {
            cmd.args(args);
        }
        if let Some(envs) = input.envs {
            cmd.envs(envs);
        }
        if let Some(cwd) = input.cwd {
            cmd.current_dir(cwd);
        }
        if input.input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut handle = cmd.spawn()?;
        if let Some(input) = input.input {
            if let Some(stdin) = &mut handle.stdin {
                stdin.write_all(input.as_bytes()).await?;
            } else {
                return Err(qjs::Error::Unknown);
            }
        }
        let result = handle.output().await?;
        let status = result.status.code();
        let output = String::from_utf8(result.stdout)?;
        let error = String::from_utf8(result.stderr)?;
        Ok(ExecRes {
            status,
            output,
            error,
        })
    }
}
