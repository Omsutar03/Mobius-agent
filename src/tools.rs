use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, PartialEq)]
pub enum ToolCall {
    // Direct bash/shell command execution (e.g., `bash ls -la`)
    Shell(String),
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

impl CommandOutput {
    // Formats the result into a clean, single string for LLM to read.
    pub fn to_llm_string(&self) -> String {
        let status_str = if self.success { "SUCCESS" } else { "FAILED" };
        let code_str = self
            .exit_code
            .map_or("Unknown".to_string(), |c| c.to_string());

        let mut output = format!(
            "[Command Execution: {} (Exit Code: {})]\n",
            status_str, code_str
        );

        if !self.stdout.trim().is_empty() {
            output.push_str("--- STDOUT ---\n");
            output.push_str(&self.stdout);
            output.push('\n');
        }

        if !self.stderr.trim().is_empty() {
            output.push_str("--- STDERR ---\n");
            output.push_str(&self.stderr);
            output.push('\n');
        }

        output
    }
}

pub fn parse_tool_call(text: &str) -> Option<ToolCall> {
    for line in text.lines() {
        let trimmed = line.trim();

        // Detect command syntax starting with `bash`
        if let Some(cmd) = trimmed.strip_prefix("bash ") {
            return Some(ToolCall::Shell(cmd.trim().to_string()));
        }
    }

    None
}

pub async fn execute_shell_command(
    command_str: &str,
    timeout_secs: u64,
) -> Result<CommandOutput, String> {
    // Spawn chlid process
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command_str);

    // CRITICAL: Automatically send SIGKILL to child process if timeout drops
    cmd.kill_on_drop(true);

    let duration = Duration::from_secs(timeout_secs);

    match timeout(duration, cmd.output()).await {
        Ok(Ok(output)) => Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
            exit_code: output.status.code(),
        }),
        Ok(Err(e)) => Err(format!("Failed to execute process: {}", e)),
        Err(_) => Err(format!(
            "❌ Execution timed out after {} seconds. Process terminated.",
            timeout_secs
        )),
    }
}
