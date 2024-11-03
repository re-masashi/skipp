use frontend::generator::Generator;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use frontend::{init_cli, init_logger};
use log::error;
use log::warn;
use std::fs;
use std::process;

/// Unwrap and return result, or log and exit if Err.
macro_rules! unwrap_or_exit {
    ($f:expr, $origin:tt) => {
        match $f {
            Ok(a) => a,
            Err(e) => {
                error!("{}: {}", $origin, e);
                process::exit(1);
            }
        }
    };
}

pub fn main() {
    let cli_input = init_cli();
    init_logger(cli_input.verbose);

    // Lexer
    let lexer = unwrap_or_exit!(Lexer::from_file(&cli_input.input_path), "IO");
    let tokens = lexer
        .map(|t| unwrap_or_exit!(t, "Lexing"))
        .collect::<Vec<_>>();

    if cli_input.print_tokens {
        println!("***TOKENS***");
        tokens.iter().for_each(|t| println!("{:?}", t));
    }

    // Parser
    let mut parser = Parser::new(tokens.into_iter().peekable(), &cli_input.input_path);
    let program = unwrap_or_exit!(parser.parse_program(), "Parsing");
    if !true {
        println!("***AST***\n{:#?}", program);
    }

    let generator = unsafe { Generator::new(program, &cli_input.input_name) };
    unsafe {
        generator.init();
        unwrap_or_exit!(generator.generate(), "Code Generation");
        // unwrap_or_exit!(generator.verify(), "LLVM");
        // generator.optimize();

        let object_file = format!("{}.o", cli_input.input_name);

        unwrap_or_exit!(
            generator.generate_ir(format!("{}.ir", cli_input.input_name).as_str()),
            "LLVM"
        );
        unwrap_or_exit!(generator.generate_object_file(3, &object_file), "LLVM");
        unwrap_or_exit!(
            generator.generate_executable(&object_file, &cli_input.output_path),
            "Linker"
        );
        // fs::remove_file(object_file).unwrap_or_else(|e| {
        //     warn!("Unable to delete object file:\n{}", e);
        // });
    }
}
