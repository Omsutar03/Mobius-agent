use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;

use html_to_markdown_rs::convert;
use readabilityrs::{Readability, ReadabilityOptions};
use scraper::{Html, Selector};

#[derive(Debug, PartialEq)]
pub enum ToolCall {
    // Direct bash/shell command execution
    Shell(String),

    // File reading tool
    Read {
        path: String,
    },

    // File creation and overwrite tool
    Write {
        path: String,
        content: String,
    },

    // File edit tool
    Edit {
        path: String,
        old_text: String,
        new_text: String,
    },

    // Seach the web with query
    WebSearch {
        query: String,
    },

    // Reads web-page tool
    ReadWebPage {
        url: String,
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
            } else if header == "write" {
                // Check for write tool (Line 1 = path, Line 2+ = content)
                let mut path = String::new();
                let mut content = String::new();
                let mut is_first_line = true;

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    if is_first_line {
                        path = line.trim().to_string();
                        is_first_line = false;
                    } else {
                        content.push_str(line);
                        content.push('\n');
                    }
                }

                // Execution of 'write' tool
                if !path.is_empty() {
                    return Some(ToolCall::Write { path, content });
                }
            } else if header == "edit" {
                // Check for edit tool (Line 1 = path, Line 2+ = search/replace content)
                let mut path = String::new();
                let mut content = String::new();
                let mut is_first_line = true;

                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    if is_first_line {
                        path = line.trim().to_string();
                        is_first_line = false;
                    } else {
                        content.push_str(line);
                        content.push('\n');
                    }
                }

                // Execution of 'edit' tool
                if let Some((old_text, new_text)) = parse_edit_markers(&content) {
                    return Some(ToolCall::Edit {
                        path,
                        old_text,
                        new_text,
                    });
                }
            } else if header == "web_search" {
                let mut query = String::new();
                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    query.push_str(line.trim());
                    query.push(' '); // allow multi-line queries just in case
                }

                let final_query = query.trim().to_string();
                if !final_query.is_empty() {
                    return Some(ToolCall::WebSearch { query: final_query });
                }
            } else if header == "read_webpage" {
                let mut url = String::new();
                for line in lines.by_ref() {
                    if line.trim().starts_with("```") {
                        break;
                    }
                    url.push_str(line.trim());
                }

                if !url.is_empty() {
                    return Some(ToolCall::ReadWebPage { url });
                }
            }
        }
    }

    None
}

fn parse_edit_markers(content: &str) -> Option<(String, String)> {
    // Check for Git-style SEARCH / REPLACE markers (Aider format)
    if let (Some(s_start), Some(s_mid), Some(s_end)) = (
        content.find("<<<<<<< SEARCH"),
        content.find("======="),
        content.find(">>>>>>> REPLACE"),
    ) {
        if s_start < s_mid && s_mid < s_end {
            let old_text = content[s_start + 14..s_mid].trim_matches('\n').to_string();
            let new_text = content[s_mid + 7..s_end].trim_matches('\n').to_string();

            if !old_text.is_empty() {
                return Some((old_text, new_text));
            }
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

        output.push_str(line);
        output.push('\n');
        line_count += 1;
    }

    if line_count == 0 {
        output.push_str("File is Empty!\n");
    }

    output
}

pub async fn execute_write_command(path: &str, content: &str) -> String {
    let expanded_path = expand_path(path);
    let path_obj = Path::new(&expanded_path);

    // 1. Create parent directories if they don't exist yet
    if let Some(parent) = path_obj.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return format!(
                    "❌ [Write Error]: Failed to create directories for '{}'. Reason: {}",
                    expanded_path, e
                );
            }
        }
    }

    // 2. Write the content asynchronously to disk
    match fs::write(&expanded_path, content).await {
        Ok(_) => {
            let bytes = content.len();
            format!(
                "✅ [Write Success]: Successfully wrote {} bytes to '{}'.",
                bytes, expanded_path
            )
        }
        Err(e) => format!(
            "❌ [Write Error]: Failed to write to file '{}'. Reason: {}",
            expanded_path, e
        ),
    }
}

pub async fn execute_edit_command(path: &str, old_text: &str, new_text: &str) -> String {
    let expanded_path = expand_path(path);

    // 1. Read existing file contents (not feeding this to LLM, this is just to cross-verify file's content before editing)
    let content = match fs::read_to_string(&expanded_path).await {
        Ok(c) => c,
        Err(e) => {
            return format!(
                "❌ [Edit Error]: Failed to read target file '{}'. Reason: {}",
                expanded_path, e
            );
        }
    };

    // 2. Check if target text exists in file
    if !content.contains(old_text) {
        return format!(
            "❌ [Edit Error]: The text block to replace was not found in '{}'. \n\
            💡 Tip: Use 'read' tool first to check exact indentation, whitespace, and line content.",
            expanded_path
        );
    }

    // 3. Check for multiple occurrences
    let occurrences = content.matches(old_text).count();
    let updated_content = content.replacen(old_text, new_text, 1);

    // 4. Save updated content back to disk
    match fs::write(&expanded_path, updated_content).await {
        Ok(_) => {
            if occurrences > 1 {
                format!(
                    "✅ [Edit Success]: Replaced 1st of {} matching occurrences in '{}'.",
                    occurrences, expanded_path
                )
            } else {
                format!(
                    "✅ [Edit Success]: File '{}' updated successfully.",
                    expanded_path
                )
            }
        }
        Err(e) => format!(
            "❌ [Edit Error]: Failed to write changes to '{}'. Reason: {}",
            expanded_path, e
        ),
    }
}

pub async fn execute_web_search_command(query: &str) -> String {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    // Properly initialized client with global user agent
    let client = match reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("❌ [WebSearch Error]: Failed to build HTTP client: {}", e),
    };

    let resp = match client.get(&url).send().await {
        Ok(r) => match r.text().await {
            Ok(text) => text,
            Err(e) => return format!("❌ [WebSearch Error]: Failed to read response body: {}", e),
        },
        Err(e) => return format!("❌ [WebSearch Error]: Request failed: {}", e),
    };

    let document = Html::parse_document(&resp);

    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__title").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();
    let url_selector = Selector::parse(".result__url").unwrap();

    let mut summary = format!("[Search Results for '{}']\n\n", query);
    let mut count = 0;

    for element in document.select(&result_selector).take(5) {
        let title = element
            .select(&title_selector)
            .next()
            .map_or("No Title".to_string(), |el| {
                el.text().collect::<Vec<_>>().join("").trim().to_string()
            });
        let snippet = element
            .select(&snippet_selector)
            .next()
            .map_or("No Snippet".to_string(), |el| {
                el.text().collect::<Vec<_>>().join("").trim().to_string()
            });
        let raw_url = element
            .select(&url_selector)
            .next()
            .map_or("".to_string(), |el| {
                el.text().collect::<Vec<_>>().join("").trim().to_string()
            });

        if !title.is_empty() {
            count += 1;
            summary.push_str(&format!(
                "{}. {}\n   URL: {}\n   Snippet: {}\n\n",
                count, title, raw_url, snippet
            ));
        }
    }

    if count == 0 {
        return format!(
            "❌ [WebSearch Error]: No search results found for query '{}'",
            query
        );
    }

    summary
}

pub async fn execute_read_webpage_command(url: &str) -> String {
    let clean_url = url.trim();

    // 1. Fetch raw HTML
    let raw_html = match reqwest::get(clean_url).await {
        Ok(res) => match res.text().await {
            Ok(text) => text,
            Err(e) => {
                return format!(
                    "❌ [ReadPage Error]: Failed to read text from {}. Reason: {}",
                    clean_url, e
                );
            }
        },
        Err(e) => {
            return format!(
                "❌ [ReadPage Error]: Failed to reach {}. Reason: {}",
                clean_url, e
            );
        }
    };

    // 2. Pass 1: Readability Boilerplate Removal
    let options = ReadabilityOptions::default();
    let readability = match Readability::new(&raw_html, Some(clean_url), Some(options)) {
        Ok(r) => r,
        Err(e) => {
            return format!(
                "❌ [ReadPage Error]: Readability initialization failed: {}",
                e
            );
        }
    };

    if let Some(article) = readability.parse() {
        if let Some(clean_html) = article.content {
            // 3. Pass 2: Convert pure HTML content left behing to Markdown
            match convert(&clean_html, None) {
                Ok(conversion_result) => {
                    if let Some(markdown) = conversion_result.content {
                        let mut output = format!("---[Page Content for {}]---\n\n", clean_url);
                        output.push_str(&markdown);

                        return output;
                    }
                }
                Err(e) => return format!("❌ [ReadPage Error]: Markdown conversion failed: {}", e),
            }
        }
    }

    // Fallback if readability strips everything (e.g., heavily JS rendered sites)
    "❌ [ReadPage Error]: Could not identify or extract main content block from this page."
        .to_string()
}
