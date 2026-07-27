// TODO: -t and -m value checks
use pico_args::{Arguments, Error};

#[derive(Debug, PartialEq)]
pub enum FlagAction {
    Query,
    Set(String),
}

#[derive(Debug)]
pub struct CliArgs {
    pub new_session: bool,
    pub thinking_level: Option<FlagAction>,
    pub model: Option<FlagAction>,
    pub last_lines: Option<usize>,
    pub prompt: String,
}

impl CliArgs {
    pub fn parse() -> Result<Self, String> {
        let mut par = Arguments::from_env();

        // 1. Extract all flags
        let new_session = par.contains(["-n", "--new-s"]); // For new chat session in same TTY session.
        let thinking_level = parse_flag_action(&mut par, ["-t", "--thinking"])?; // For toggling thinking level/mode
        let model = parse_flag_action(&mut par, ["-m", "--model"])?; // For checking current model or changing model
        let last_lines = par
            .opt_value_from_str(["-l", "--last"])
            .map_err(|e| e.to_string())?; // To let mobius access "N" last i/o of terminal

        // 2. Finish parsing to consume remaining args as prompt
        let remaining_args = par.finish();

        let prompt_words: Vec<String> = remaining_args
            .into_iter()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .collect();

        let prompt = prompt_words.join(" ");

        // 3. Security checks
        // A: Only one flag should be present at a time
        let flag_count = [
            new_session,
            thinking_level.is_some(),
            model.is_some(),
            last_lines.is_some(),
        ]
        .iter()
        .filter(|&&is_set| is_set)
        .count();

        if flag_count > 1 {
            return Err(
                "Multiple flags provided! Mobius only allows 1 flag per command.".to_string(),
            );
        }

        // B: -n / --new-s MUST be standalone (no prompt allowed)
        if new_session && !prompt.is_empty() {
            return Err(
                "The `-n` / `--new-s` flag must be used on its own. Do not pass a prompt with it."
                    .to_string(),
            );
        }

        // C: -t / --thinking MUST be standalone / value-only (no prompt allowed)
        if thinking_level.is_some() && !prompt.is_empty() {
            return Err(
                "The `-t` / `--thinking` flag is strictly for setting or querying thinking levels. Do not pass a prompt with it.".to_string(),
            );
        }

        // D: -m / --model MUST be standalone / value-only (no prompt allowed)
        if model.is_some() && !prompt.is_empty() {
            return Err("The model flag (-m / --model) is strictly for querying or switching models. Do not pass a prompt with it.".to_string());
        }

        Ok(CliArgs {
            new_session,
            thinking_level,
            model,
            last_lines,
            prompt,
        })
    }
}

fn parse_flag_action(
    par: &mut Arguments,
    keys: [&'static str; 2],
) -> Result<Option<FlagAction>, String> {
    match par.opt_value_from_str(keys) {
        Ok(Some(val)) => Ok(Some(FlagAction::Set(val))),
        Ok(None) => Ok(None),
        Err(Error::OptionWithoutAValue(_)) => {
            // Consumes and removes the orphaned flag from `par` so it won't leak into `prompt`
            let _ = par.contains(keys);
            Ok(Some(FlagAction::Query))
        }
        Err(err) => Err(err.to_string()),
    }
}
