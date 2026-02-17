use std::process::{Command, Stdio, ExitStatus};

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

        cmd.status().map_err(|e| format!("Failed to run {}: {}", program, e))
    }
}
