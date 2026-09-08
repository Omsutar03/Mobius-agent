// TODO: -t and -m value checks
use pico_args::{Arguments, Error};

#[derive(Debug, PartialEq)]
pub enum FlagAction {
    Query,
    Set(String),
}

#[derive(Debug)]
pub struct CliArgs {
    pub help: bool,
    pub daemon_mode: bool, // Hidden flag
    pub stop_daemon: bool,
    pub new_session: bool,
    pub gui: bool,
    pub session: Option<FlagAction>,
    pub tokens: bool,
    pub thinking_level: Option<FlagAction>,
    pub model: Option<FlagAction>,
    pub last_lines: Option<usize>,
    pub prompt: String,
    pub record_cmd: Option<String>, // Hidden flag
    pub override_pid: Option<u32>,  // Hidden flag
}

impl CliArgs {
    pub fn parse() -> Result<Self, String> {
        let mut par = Arguments::from_env();

        // Instant Help check (Bypasses all other validation checks)
        let help = par.contains(["-h", "--help"]);
        if help {
            return Ok(CliArgs {
                help: true,
                daemon_mode: false,
                stop_daemon: false,
                new_session: false,
                gui: false,
                tokens: false,
                thinking_level: None,
                model: None,
                session: None,
                last_lines: None,
                prompt: String::new(),
                record_cmd: None,
                override_pid: None,
            });
        }

        // Check for daemon mode first
        let daemon_mode = par.contains("--daemon-mode");

        // Flag for stopping daemon
        let stop_daemon = par.contains("--stop-daemon");

        // Flag for launching GUI
        let gui = par.contains("--gui");

        // 1. Extract all flags
        let help = par.contains(["-h", "--help"]);
        let new_session = par.contains(["-n", "--new-s"]); // For new chat session in same TTY session.
        let tokens = par.contains("--tokens"); // For checking token count
        let thinking_level = parse_flag_action(&mut par, ["-t", "--thinking"])?; // For toggling thinking level/mode
        let model = parse_flag_action(&mut par, ["-m", "--model"])?; // For checking current model or changing model
        let session = parse_flag_action(&mut par, ["-s", "--session"])?; // For loading past sessions into current one
        let last_lines = par
            .opt_value_from_str(["-l", "--last"])
            .map_err(|e| e.to_string())?; // To let mobius access "N" last i/o of terminal

        // Extract hidden flags
        let record_cmd: Option<String> = par.opt_value_from_str("--record").unwrap_or(None);
        let override_pid: Option<u32> = par.opt_value_from_str("--pid").unwrap_or(None);

        // 2. Finish parsing to consume remaining args as prompt
        let remaining_args = par.finish();

        let prompt_words: Vec<String> = remaining_args
            .into_iter()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .collect();

        let prompt = prompt_words.join(" ");
        let has_prompt = !prompt.is_empty();

        // If running in daemon mode, bypass normal CLI validation
        if daemon_mode {
            return Ok(CliArgs {
                help: false,
                daemon_mode,
                stop_daemon: false,
                new_session: false,
                gui: false,
                session: None,
                tokens: false,
                thinking_level: None,
                model: None,
                last_lines: None,
                prompt: String::new(),
                record_cmd,
                override_pid,
            });
        }

        // 3. Security checks
        // A: Only one flag should be present at a time
        let flag_count = [
            help,
            stop_daemon,
            new_session,
            gui,
            tokens,
            thinking_level.is_some(),
            model.is_some(),
            last_lines.is_some(),
            session.is_some(),
        ]
        .iter()
        .filter(|&&is_set| is_set)
        .count();

        if flag_count > 1 {
            return Err(
                "Multiple flags provided! Mobius only allows 1 flag per command.".to_string(),
            );
        }

        // B: --gui MUST be standalone
        if gui && has_prompt {
            return Err(
                "The `--gui` flag must be used on its own. Do not pass a prompt.".to_string(),
            );
        }

        // C: --stop-daemon MUST be standalone (no prompt allowed)
        if stop_daemon && has_prompt {
            return Err(
                "The `--stop-daemon` flag must be used on its own. Do not pass a prompt."
                    .to_string(),
            );
        }

        // D: -n / --new-s MUST be standalone (no prompt allowed)
        if new_session && has_prompt {
            return Err(
                "The `-n` / `--new-s` flag must be used on its own. Do not pass a prompt with it."
                    .to_string(),
            );
        }

        // E: -t / --thinking MUST be standalone / value-only (no prompt allowed)
        if thinking_level.is_some() && has_prompt {
            return Err(
                "The `-t` / `--thinking` flag is strictly for setting or querying thinking levels. Do not pass a prompt with it.".to_string(),
            );
        }

        // F: -m / --model MUST be standalone / value-only (no prompt allowed)
        if model.is_some() && has_prompt {
            return Err("The model flag (-m / --model) is strictly for querying or switching models. Do not pass a prompt with it.".to_string());
        }

        // G: --tokens MUST be standalone
        if tokens && has_prompt {
            return Err(
                "The `--tokens` flag must be used on its own. Do not pass a prompt with it."
                    .to_string(),
            );
        }

        // H: MUST be standalone
        if session.is_some() && has_prompt {
            return Err(
                "The `-s` / `--session` flag is strictly for querying or switching sessions. Do not pass a prompt with it.".to_string(),
            );
        }

        Ok(CliArgs {
            help,
            daemon_mode,
            stop_daemon,
            new_session,
            gui,
            session,
            tokens,
            thinking_level,
            model,
            last_lines,
            prompt,
            record_cmd,
            override_pid,
        })
    }
}

pub fn parse_flag_action(
    par: &mut Arguments,
    keys: [&'static str; 2],
) -> Result<Option<FlagAction>, String> {
    match par.opt_value_from_str(keys) {
        Ok(Some(val)) => Ok(Some(FlagAction::Set(val))),
        Ok(None) => Ok(None),
        Err(Error::OptionWithoutAValue(_)) => {
            let _ = par.contains(keys);
            Ok(Some(FlagAction::Query))
        }
        Err(err) => Err(err.to_string()),
    }
}

/// Print formatted ANSI color help menu
pub fn print_help() {
    println!(
        "\x1B[1;36mMobius\x1B[0m - Terminal-based AI Agent Harness\n\n\
        \x1B[1;33mUSAGE:\x1B[0m\n  \
          mobius [FLAGS] [PROMPT]\n\n\
        \x1B[1;33mFLAGS:\x1B[0m\n  \
          \x1B[1;32m-h, --help\x1B[0m              Show this help message\n  \
          \x1B[1;32m--gui\x1B[0m                   Launch the desktop GUI frontend\n  \
          \x1B[1;32m-n, --new-s\x1B[0m             Archive current session & start fresh\n  \
          \x1B[1;32m-s, --session [PID]\x1B[0m     Launch interactive session browser (TUI) or switch session PID\n  \
          \x1B[1;32m-m, --model [NAME]\x1B[0m      Query available local models or set provider (llama, ollama, lmstudio)\n  \
          \x1B[1;32m-t, --thinking [LVL]\x1B[0m    Query or set thinking level (off, min, low, med, high, xhigh, max)\n  \
          \x1B[1;32m-l, --last <N>\x1B[0m          Inject output of last N shell commands into prompt context\n  \
          \x1B[1;32m--tokens\x1B[0m                Check current session context token usage\n  \
          \x1B[1;32m--stop-daemon\x1B[0m           Stop the background Mobius daemon process\n\n\
        \x1B[1;33mEXAMPLES:\x1B[0m\n  \
          mobius \"Explain Tokio async channels\"\n  \
          mobius --gui\n  \
          mobius -l 3 \"Why did my cargo build fail?\"\n  \
          mobius -s\n  \
          mobius -t high"
    );
}
