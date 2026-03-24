use std::io::Write;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Hooks {
    #[serde(default)]
    pub record: Vec<HookEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookEntry(toml::Value);

#[derive(Debug, Serialize, Deserialize)]
struct RawHook {
    command: String,
    args: Vec<String>,
}

impl HookEntry {
    pub fn run(&self, path: PathBuf) -> Result<(), anyhow::Error> {
        let (proc, s) = match &self.0 {
            toml::Value::String(s) => {
                if s.is_empty() {
                    return Ok(());
                }
                (
                    if cfg!(target_os = "windows") {
                        std::process::Command::new("cmd")
                            .current_dir(path)
                            .args(&["/C", s])
                            .output()
                            .expect("failed to execute process")
                    } else {
                        std::process::Command::new(
                            std::env::var("SHELL").unwrap_or("sh".to_string()),
                        )
                        .current_dir(path)
                        .arg("-c")
                        .arg(s)
                        .output()
                        .expect("failed to execute process")
                    },
                    s.clone(),
                )
            }
            v => {
                let hook = v.clone().try_into::<RawHook>()?;
                (
                    std::process::Command::new(&hook.command)
                        .current_dir(path)
                        .args(&hook.args)
                        .output()
                        .expect("failed to execute process"),
                    hook.command,
                )
            }
        };
        if !proc.status.success() {
            let mut stderr = std::io::stderr();
            writeln!(stderr, "Hook {:?} exited with code {:?}", s, proc.status)?;
            std::process::exit(proc.status.code().unwrap_or(1))
        }
        Ok(())
    }
}
