// TODO: `-l` or `--last` flag
mod cli;
mod engine;

use cli::{CliArgs, FlagAction};
use engine::{ThinkingLevel, ask_mobius};
use reqwest::Client;

#[tokio::main]
async fn main() {
    match CliArgs::parse() {
        Ok(args) => {
            // For now, if the user provides a prompt, we just send it to the engine directly.
            if !args.prompt.is_empty() {
                // Determine the thinking level for this request
                let thinking_level =
                    ThinkingLevel::from_str("medium").unwrap_or(ThinkingLevel::Off);

                // Hardcoded for testing, will move this to config.rs later
                let server_url = "http://localhost:8080/v1/chat/completions";
                let client = Client::new();

                match ask_mobius(&client, server_url, &args.prompt, thinking_level).await {
                    Ok(_) => {} // Stream is already printed to stdout
                    Err(e) => eprintln!("❌ Mobius Engine Error: {}", e),
                }

                return;
            }

            // `-n` or `--new-s` flag
            if args.new_session {
                println!("🧹 Initializing new session..."); // Placeholder
            }

            // `-t` or `--thinking` flag
            match &args.thinking_level {
                Some(FlagAction::Query) => {
                    println!("Thinking Level (-t / --thinking): [QUERY MODE]");
                    println!("  💡 Current Level: Off (Default)");
                    println!("  💡 Available Options: off, minimal, low, medium, high, xhigh, max");
                }
                _ => {}
            }

            // '-m' or '--model' flag
            match &args.model {
                Some(FlagAction::Query) => {
                    println!("Model (-m / --model)             : [QUERY MODE]");
                    println!("  🤖 Current Model: local (Default)");
                    println!("  🤖 Available Options: local, gemini, claude");
                }
                _ => {}
            }
        }
        Err(err) => {
            eprintln!("❌ Error: {}", err);
        }
    }
}
