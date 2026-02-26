use busbar_sf_agentscript_parser::lexer::lex_with_indentation;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.agent> [target_line]", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let source = fs::read_to_string(filename).expect("Failed to read file");

    // Get line from arg if provided
    let target_line: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    match lex_with_indentation(&source) {
        Ok(tokens) => {
            for (tok, span) in &tokens {
                // Calculate line number from span
                let line = source[..span.start].matches('\n').count() + 1;

                // Print tokens around target line
                if target_line == 0
                    || (line >= target_line.saturating_sub(5) && line <= target_line + 5)
                {
                    println!("Line {:4}: {:?} @ {:?}", line, tok, span);
                }
            }
        }
        Err(e) => eprintln!("Lexer error: {:?}", e),
    }
}
