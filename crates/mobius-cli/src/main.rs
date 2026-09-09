mod cli;
mod ipc_client;
mod tui;

use mobius_core::ipc::IpcRequest;
use mobius_core::session::{Session, TerminalHistory};

#[tokio::main]
async fn main() {
    // 1. Parse arguments using your exact CliArgs struct
    let args = match cli::CliArgs::parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("❌ Argument Error: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Handle Instant Help
    if args.help {
        cli::print_help();
        return;
    }

    // --- HANDLE --gui FLAG ---
    if args.gui {
        println!("🚀 Launching Mobius GUI...");

        // Look for the mobius-gui binary next to current executable target directory
        let current_exe = std::env::current_exe().ok();
        let gui_binary = current_exe
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("mobius-gui"))
            .unwrap_or_else(|| std::path::PathBuf::from("mobius-gui"));

        match std::process::Command::new(gui_binary).spawn() {
            Ok(_) => {
                println!("✨ Mobius GUI launched successfully.");
                return;
            }
            Err(e) => {
                eprintln!("❌ Failed to launch mobius-gui process: {}", e);
                eprintln!("💡 Tip: Make sure to build the GUI using: cargo build -p mobius-gui");
                std::process::exit(1);
            }
        }
    }

    let ppid = args.override_pid.unwrap_or_else(std::process::id);

    // --- HANDLE -s / --session (TUI Session Browser or Switching) ---
    if let Some(ref flag) = args.session {
        match flag {
            cli::FlagAction::Query => {
                let sessions = Session::list_all_sessions(ppid);
                if sessions.is_empty() {
                    println!("No sessions found.");
                    return;
                }
                match tui::run_tui(sessions) {
                    Ok(Some(selected_path)) => {
                        Session::load_from_path_and_overwrite_current(&selected_path, ppid);
                        println!("✅ Switched to session: {}", selected_path.display());
                        return;
                    }
                    Ok(None) => return,
                    Err(e) => {
                        eprintln!("TUI Error: {}", e);
                        return;
                    }
                }
            }
            cli::FlagAction::Set(target_pid_str) => {
                if let Ok(target_pid) = target_pid_str.parse::<u32>() {
                    // Attempt to load the targeted PID's file and overwrite the current one
                    let target_path = mobius_core::session::Session::get_session_dir()
                        .join(format!("{}.json", target_pid));
                    if target_path.exists() {
                        Session::load_from_path_and_overwrite_current(&target_path, ppid);
                        println!("✅ Switched to session PID: {}", target_pid);
                    } else {
                        eprintln!("❌ Session file for PID {} does not exist.", target_pid);
                    }
                    return;
                } else {
                    eprintln!("❌ Invalid PID provided for --session");
                    return;
                }
            }
        }
    }

    // --- HANDLE -l / --last <N> (Inject recent command history) ---
    let mut final_prompt = args.prompt.clone();
    if let Some(n) = args.last_lines {
        let history = TerminalHistory::load(ppid);
        let recent_entries = history.get_last_n(n);

        if !recent_entries.is_empty() {
            let mut recent_str = String::new();
            for entry in recent_entries {
                recent_str.push_str(&format!("$ {}\n{}\n", entry.command, entry.output));
            }
            final_prompt = format!(
                "[Context: Last {} Terminal Commands]\n{}\n\nUser Question: {}",
                n, recent_str, final_prompt
            );
        }
    }

    // --- EXTRACT FLAG ACTIONS ---
    let thinking_override = match &args.thinking_level {
        Some(cli::FlagAction::Set(val)) => Some(val.clone()),
        _ => None,
    };
    let query_thinking = matches!(&args.thinking_level, Some(cli::FlagAction::Query));

    let model_override = match &args.model {
        Some(cli::FlagAction::Set(val)) => Some(val.clone()),
        _ => None,
    };
    let query_model = matches!(&args.model, Some(cli::FlagAction::Query));

    // 3. Build the payload for the daemon
    let request = IpcRequest {
        ppid,
        prompt: final_prompt,
        thinking_level_override: thinking_override,
        model_override,
        new_session: args.new_session,
        query_tokens: args.tokens,
        query_model,
        query_thinking,
        shutdown: args.stop_daemon,
        record_history: None,
        list_sessions: false,
        load_session: None,
        delete_session: None,
    };

    // 5. Exit early if no action/prompt is specified
    let has_action = !request.prompt.is_empty()
        || request.shutdown
        || request.new_session
        || request.query_tokens
        || request.query_model
        || request.query_thinking
        || request.thinking_level_override.is_some()
        || request.model_override.is_some();

    if !has_action {
        cli::print_help();
        return;
    }

    // 6. Send request over WebSocket to the daemon
    if let Err(e) = ipc_client::send_to_daemon(request).await {
        eprintln!("{}", e);
        eprintln!("Please start the daemon in another terminal with: cargo run -p mobius-daemon");
    }
}

// async fn handle_queries(args: &CliArgs) {
//     let ppid = args
//         .override_pid
//         .unwrap_or_else(|| std::os::unix::process::parent_id());

//     // Update new session behavior to archive
//     if args.new_session {
//         session::Session::archive_and_reset(ppid);
//         println!("🧹 Archived previous messages and initialized a fresh session.");
//     }

//     // New Session Selector Logic
//     if let Some(ref action) = args.session {
//         match action {
//             cli::FlagAction::Query => {
//                 let list = session::Session::list_all_sessions(ppid);
//                 if list.is_empty() {
//                     println!("⚠️ No saved sessions found on disk.");
//                     return;
//                 }

//                 // Spawn TUI
//                 match crate::tui::run_tui(list) {
//                     Ok(Some(selected_path)) => {
//                         session::Session::load_from_path_and_overwrite_current(
//                             &selected_path,
//                             ppid,
//                         );
//                         println!("✅ Session loaded successfully. You can now chat to continue.");
//                     }
//                     Ok(None) => {
//                         println!("🛑 Session selection cancelled.");
//                     }
//                     Err(e) => {
//                         eprintln!("❌ TUI Error: {}", e);
//                     }
//                 }
//             }
//             cli::FlagAction::Set(val) => {
//                 // Allows bypass for power-users who want to load a known PID string without UI
//                 let target_file = session::Session::get_session_dir().join(format!("{}.json", val));
//                 if target_file.exists() {
//                     session::Session::load_from_path_and_overwrite_current(&target_file, ppid);
//                     println!("✅ Loaded session '{}' successfully.", val);
//                 } else {
//                     eprintln!("❌ Error: Session file '{}.json' not found.", val);
//                 }
//             }
//         }
//         return;
//     }

//     if args.tokens {
//         let session = session::Session::load(ppid);

//         let used = session.last_prompt_tokens + session.last_completion_tokens;
//         let total = if session.context_window > 0 {
//             session.context_window
//         } else {
//             8192
//         };
//         let pct = if total > 0 {
//             (used as f64 / total as f64) * 100.0
//         } else {
//             0.0
//         };

//         // Dynamic warning colors based on usage capacity
//         let pct_color = if pct > 85.0 {
//             "\x1B[1;31m" // Bold Red
//         } else if pct > 60.0 {
//             "\x1B[1;33m" // Bold Yellow
//         } else {
//             "\x1B[1;32m" // Bold Green
//         };

//         println!(
//             "\x1B[1;36m{}\x1B[0m\x1B[2m/\x1B[0m\x1B[36m{}\x1B[0m | {}{:.2}%\x1B[0m | \x1B[1;35mInput:\x1B[0m \x1B[33m{}\x1B[0m | \x1B[1;35mOutput:\x1B[0m \x1B[33m{}\x1B[0m",
//             used, total, pct_color, pct, session.last_prompt_tokens, session.last_completion_tokens
//         );
//         return;
//     }

//     if let Some(n) = args.last_lines {
//         let history = session::TerminalHistory::load(ppid);
//         let last_n = history.get_last_n(n);

//         println!("Terminal History (-l / --last): [PID {}]", ppid);
//         if last_n.is_empty() {
//             println!("  ⚠️ No history recorded yet.");
//         } else {
//             for (i, entry) in last_n.iter().enumerate() {
//                 println!("\n--- [{}] $ {} ---", i + 1, entry.command);
//                 println!("{}", entry.output.trim());
//             }
//         }
//         return;
//     }

//     if let Some(ref action) = args.thinking_level {
//         let mut session = session::Session::load(ppid);

//         match action {
//             cli::FlagAction::Query => {
//                 let current = session.thinking_level.as_deref().unwrap_or("off");
//                 println!("Thinking Level (-t / --thinking): [QUERY MODE]");
//                 println!("  💡 Current Level: {}", current);
//                 println!(
//                     "  💡 Available Options: off, min/minimal, low, med/medium, high, xhigh, max"
//                 );
//             }
//             cli::FlagAction::Set(val) => {
//                 if engine::ThinkingLevel::from_str(val).is_some() {
//                     session.thinking_level = Some(val.clone());
//                     session.save(ppid);
//                     println!(
//                         "✅ Thinking level set to '{}' for current session (PID {}).",
//                         val, ppid
//                     );
//                 } else {
//                     eprintln!("❌ Invalid thinking level '{}'.", val);
//                     eprintln!(
//                         "  💡 Available options: off, min/minimal, low, med/medium, high, xhigh, max"
//                     );
//                 }
//             }
//         }
//         return;
//     }

//     match &args.model {
//         Some(FlagAction::Query) => {
//             let ppid = args
//                 .override_pid
//                 .unwrap_or_else(|| std::os::unix::process::parent_id());
//             let session = session::Session::load(ppid);
//             let current_provider = session.model_provider.as_deref().unwrap_or("llama");

//             println!("Model (-m / --model)             : [QUERY MODE]");
//             println!("  💡 Current Provider            : {}", current_provider);
//             println!("  🔍 Scanning local ports for active inference engines...\n");
//             let active_models = crate::inferences::discover_local_models().await;

//             if active_models.is_empty() {
//                 println!(
//                     "  ⚠️ No active local models found. Start Ollama, llama.cpp, or LM Studio first."
//                 );
//             } else {
//                 for (i, info) in active_models.iter().enumerate() {
//                     // Check if this engine matches the currently configured session provider
//                     let is_current = match (current_provider, &info.engine) {
//                         ("ollama", crate::inferences::InferenceEngine::Ollama) => true,
//                         ("lmstudio", crate::inferences::InferenceEngine::GenericOpenAI) => true,
//                         ("llama", crate::inferences::InferenceEngine::LlamaCpp) => true,
//                         _ => false,
//                     };

//                     let active_tag = if is_current {
//                         " \x1B[32m[Active]\x1B[0m"
//                     } else {
//                         ""
//                     };
//                     println!("  🟢 [{}] {}{}", i + 1, info.display_name, active_tag);
//                 }
//             }
//         }
//         Some(cli::FlagAction::Set(val)) => {
//             let ppid = args
//                 .override_pid
//                 .unwrap_or_else(|| std::os::unix::process::parent_id());
//             let mut session = session::Session::load(ppid);
//             session.model_provider = Some(val.to_lowercase());
//             session.save(ppid);
//             println!(
//                 "✅ Model provider set to '{}' for current session (PID {}).",
//                 val, ppid
//             );
//         }
//         _ => {}
//     }
// }
