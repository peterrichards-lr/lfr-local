#![allow(dead_code)]
use std::process::{Command, ExitStatus, Stdio};

pub struct CommandRunner;

impl CommandRunner {
    /// Executes a system command with standardized error handling and logging.
    pub fn execute(program: &str, args: &[&str], verbose: bool) -> Result<ExitStatus, String> {
        let mut cmd = Command::new(program);
        cmd.args(args);

        // If verbose is true, show the command output to the user.
        // If false, keep the terminal clean.
        if verbose {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }

        cmd.status()
            .map_err(|e| format!("Failed to run {}: {}", program, e))
    }

    pub fn force_delete(path: &std::path::Path) -> Result<(), String> {
        if path.exists() {
            if path.is_dir() {
                std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
