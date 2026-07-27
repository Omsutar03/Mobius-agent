mod cli;

use cli::{CliArgs, FlagAction};

fn main() {
    match CliArgs::parse() {
        Ok(args) => {
            println!("🚀 Mobius CLI Argument Inspection");
            println!("----------------------------------");
            // To check if flags are for new sessions
            println!("New Session (-n / --new-s)     : {}", args.new_session);

            match &args.thinking_level {
                Some(FlagAction::Query) => {
                    println!("Thinking Level (-t / --thinking): [QUERY MODE]");
                    println!("  💡 Current Level: Off (Default)");
                    println!("  💡 Available Options: off, minimal, low, medium, high, xhigh, max");
                }
                Some(FlagAction::Set(val)) => {
                    println!("Thinking Level (-t / --thinking): Set(\"{}\")", val);
                }
                None => {
                    println!("Thinking Level (-t / --thinking): None");
                }
            }

            match &args.model {
                Some(FlagAction::Query) => {
                    println!("Model (-m / --model)             : [QUERY MODE]");
                    println!("  🤖 Current Model: local (Default)");
                    println!("  🤖 Available Options: local, gemini, claude");
                }
                Some(FlagAction::Set(val)) => {
                    println!("Model (-m / --model)             : Set(\"{}\")", val);
                }
                None => {
                    println!("Model (-m / --model)             : None");
                }
            }

            println!("Last Lines (-l / --last)         : {:?}", args.last_lines);
            println!("Prompt                           : \"{}\"", args.prompt);
        }
        Err(err) => {
            eprintln!("❌ Error: {}", err);
        }
    }
}
