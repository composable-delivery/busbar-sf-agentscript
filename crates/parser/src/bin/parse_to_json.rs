//! CLI tool to parse AgentScript files and output JSON
//!
//! Usage: cargo run --bin parse_to_json <file.agent>

use busbar_sf_agentscript_parser::{parse_with_structured_errors, ErrorReporter};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.agent>", args[0]);
        eprintln!("  Parses an AgentScript file and outputs JSON to stdout");
        process::exit(1);
    }

    let filename = &args[1];

    let source = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    match parse_with_structured_errors(&source) {
        Ok(ast) => match serde_json::to_string_pretty(&ast) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("Error serializing AST to JSON: {}", e);
                process::exit(1);
            }
        },
        Err(errors) => {
            let reporter = ErrorReporter::new(filename, &source);
            for err in &errors {
                reporter.report_parse_error(err);
            }
            process::exit(1);
        }
    }
}
