use reqwest::header::CONTENT_DISPOSITION;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, PartialEq)]
pub enum ToolCall {
    // Direct bash/shell command execution (e.g. `bash ls -la`)
    Shell(String),

    // File reading tool (e.g. `read src/main.rs`)
    Read {
        path: String,
    },

    // File creation and overwrite (e.g. `write test1.py print("Hello Word!")`)
    Write {
        path: String,
        content: String,
    },

    // File edit (e.g. `edit test1.py print("Hello Word!") print("Hello World!")`)
    Edit {
        path: String,
        old_text: String,
        new_text: String,
    },
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
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            let header = trimmed.trim_start_matches('`').trim();

            if header == "bash" || header == "sh" {
                // Check for bash tool
                let mut cmd = String::new();

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    cmd.push_str(line);
                    cmd.push('\n');
                }

                // Execution of 'bash' tool
                if !cmd.trim().is_empty() {
                    return Some(ToolCall::Shell(cmd.trim().to_string()));
                }
            } else if header == "read" {
                // Check for read tool
                let mut path = String::new();

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    path.push_str(line.trim());
                }

                // Execution of 'read' tool
                if !path.is_empty() {
                    return Some(ToolCall::Read { path });
                }
            } else if let Some(path) = header.strip_prefix("write ") {
                let mut content = String::new();

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    content.push_str(line);
                    content.push('\n');
                }

                // Execution of 'write' tool
                return Some(ToolCall::Write {
                    path: path.trim().to_string(),
                    content,
                });
            } else if let Some(path) = header.strip_prefix("edit ") {
                let mut content = String::new();

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    content.push_str(line);
                    content.push('\n');
                }

                // Execution of 'edit' tool
                if let Some((old_text, new_text)) = parse_edit_markers(&content) {
                    return Some(ToolCall::Edit {
                        path: path.trim().to_string(),
                        old_text,
                        new_text,
                    });
                }
            }
        }
    }

    // for line in text.lines() {
    //     let trimmed = line.trim();

    //     // Detect command syntax starting with `bash`
    //     if let Some(cmd) = trimmed.strip_prefix("bash ") {
    //         return Some(ToolCall::Shell(cmd.trim().to_string()));
    //     }
    // }

    None
}

fn parse_edit_markers(content: &str) -> Option<(String, String)> {
    // To split the content using single seprator line
    if let Some((old_text, new_text)) = content.split_once("<===>") {
        let old = old_text.trim().trim_matches('\n').to_string();
        let new = new_text.trim().trim_matches('\n').to_string();

        if !old.is_empty() {
            return Some((old, new));
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

/// Helper function to expand tildes (`~`) to the user's home directory
fn expand_path(path: &str) -> String {
    let clean = path.trim().trim_matches('"').trim_matches('\'');
    if clean.starts_with("~/") || clean == "~" {
        if let Ok(home) = std::env::var("HOME") {
            if clean == "~" {
                return home;
            }
            return format!("{}/{}", home, &clean[2..]);
        }
    }
    clean.to_string()
}

pub async fn execute_read_command(path: &str) -> String {
    // Expand the tilde before attempting to read!
    let expanded_path = expand_path(path);

    let content = match fs::read_to_string(&expanded_path).await {
        Ok(c) => c,
        Err(e) => {
            return format!(
                "❌ [Read Error]: Failed to read '{}'. Reason: {}",
                expanded_path, e
            );
        }
    };

    let mut output = format!("[Contents of {}]\n", expanded_path);
    let max_lines = 500; // TO protect model's contex window
    let mut line_count = 0;

    for (i, line) in content.lines().enumerate() {
        if i >= max_lines {
            output.push_str(&format!(
                "\n... ⚠️ [File truncated after {} lines. Too large to read entirely.]\n",
                max_lines
            ));
            break;
        }

        // Add line numbers as prefix for edit tool
        output.push_str(&format!("{:4} | {}\n", i + 1, line));
        line_count += 1;
    }

    if line_count == 0 {
        output.push_str("File is Empty!\n");
    }

    output
}
