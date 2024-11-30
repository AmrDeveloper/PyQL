use std::fs;
use std::io;
use std::io::IsTerminal;

use arguments::Arguments;
use arguments::Command;
use gitql_cli::arguments::OutputFormat;
use gitql_cli::diagnostic_reporter;
use gitql_cli::diagnostic_reporter::DiagnosticReporter;
use gitql_cli::printer::base::OutputPrinter;
use gitql_cli::printer::csv_printer::CSVPrinter;
use gitql_cli::printer::json_printer::JSONPrinter;
use gitql_cli::printer::table_printer::TablePrinter;
use gitql_core::environment::Environment;
use gitql_core::schema::Schema;
use gitql_engine::data_provider::DataProvider;
use gitql_engine::engine;
use gitql_engine::engine::EvaluationResult::SelectedGroups;
use gitql_parser::diagnostic::Diagnostic;
use gitql_parser::parser;
use gitql_parser::tokenizer::Tokenizer;
use gitql_std::aggregation::aggregation_function_signatures;
use gitql_std::aggregation::aggregation_functions;
use gitql_std::function::standard_function_signatures;
use gitql_std::function::standard_functions;
use lineeditor::LineEditorResult;
use python::data_provider::PythonDataProvider;
use python::python_parser::parse_python_files;
use python::schema::pyql_tables_fields_names;
use python::schema::pyql_tables_fields_types;

pub mod arguments;
pub mod line_editor;
pub mod python;

fn main() {
    if cfg!(debug_assertions) {
        std::env::set_var("RUST_BACKTRACE", "1");
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    let args: Vec<String> = std::env::args().collect();
    let command = arguments::parse_arguments(&args);
    match command {
        Command::ReplMode(arguments) => {
            launch_pyql_repl(&arguments);
        }
        Command::QueryMode(query, arguments) => {
            let mut reporter = diagnostic_reporter::DiagnosticReporter::default();
            let python_modules_result = parse_python_files(&arguments.files);
            if let Err(error) = python_modules_result {
                reporter.report_diagnostic("", Diagnostic::error(&error));
                return;
            }

            let python_modules = python_modules_result.ok().unwrap();

            let schema = Schema {
                tables_fields_names: pyql_tables_fields_names(),
                tables_fields_types: pyql_tables_fields_types(),
            };

            let std_signatures = standard_function_signatures();
            let std_functions = standard_functions();

            let aggregation_signatures = aggregation_function_signatures();
            let aggregation_functions = aggregation_functions();

            let mut env = Environment::new(schema);
            env.with_standard_functions(&std_signatures, std_functions);
            env.with_aggregation_functions(&aggregation_signatures, aggregation_functions);

            let python_provider = PythonDataProvider::new(arguments.files.clone(), python_modules);
            let provider: Box<dyn DataProvider> = Box::new(python_provider);
            execute_pyql_query(query, &arguments, &mut env, &provider, &mut reporter)
        }
        Command::ScriptMode(script_file, arguments) => {
            let mut reporter = diagnostic_reporter::DiagnosticReporter::default();

            let python_modules_result = parse_python_files(&arguments.files);
            if let Err(error) = python_modules_result {
                reporter.report_diagnostic("", Diagnostic::error(&error));
                return;
            }
            let python_modules = python_modules_result.ok().unwrap();

            let queries_result = fs::read_to_string(script_file);
            if queries_result.is_err() {
                reporter.report_diagnostic("", Diagnostic::error("Can't read script file"));
                return;
            }

            let queries = queries_result.ok().unwrap();

            let schema = Schema {
                tables_fields_names: pyql_tables_fields_names(),
                tables_fields_types: pyql_tables_fields_types(),
            };

            let std_signatures = standard_function_signatures();
            let std_functions = standard_functions();

            let aggregation_signatures = aggregation_function_signatures();
            let aggregation_functions = aggregation_functions();

            let mut env = Environment::new(schema);
            env.with_standard_functions(&std_signatures, std_functions);
            env.with_aggregation_functions(&aggregation_signatures, aggregation_functions);

            let python_provider = PythonDataProvider::new(arguments.files.clone(), python_modules);
            let provider: Box<dyn DataProvider> = Box::new(python_provider);
            execute_pyql_query(queries, &arguments, &mut env, &provider, &mut reporter)
        }
        Command::Help => {
            arguments::print_help_list();
        }
        Command::Version => {
            println!("GitQL version {}", env!("CARGO_PKG_VERSION"));
        }
        Command::Error(error_message) => {
            println!("{}", error_message);
        }
    }
}

fn launch_pyql_repl(arguments: &Arguments) {
    let mut reporter = diagnostic_reporter::DiagnosticReporter::default();
    let python_modules_result = parse_python_files(&arguments.files);
    if let Err(error) = python_modules_result {
        reporter.report_diagnostic("", Diagnostic::error(&error));
        return;
    }
    let python_modules = python_modules_result.ok().unwrap();

    let schema = Schema {
        tables_fields_names: pyql_tables_fields_names(),
        tables_fields_types: pyql_tables_fields_types(),
    };

    let std_signatures = standard_function_signatures();
    let std_functions = standard_functions();

    let aggregation_signatures = aggregation_function_signatures();
    let aggregation_functions = aggregation_functions();

    let mut global_env = Environment::new(schema);
    global_env.with_standard_functions(&std_signatures, std_functions);
    global_env.with_aggregation_functions(&aggregation_signatures, aggregation_functions);

    let python_provider = PythonDataProvider::new(arguments.files.clone(), python_modules);
    let provider: Box<dyn DataProvider> = Box::new(python_provider);

    // Launch the right line editor if the flag is enabled
    // Later this line editor will be the default editor
    if arguments.enable_line_editor {
        let mut line_editor = line_editor::create_new_line_editor();
        loop {
            if let Ok(LineEditorResult::Success(input)) = line_editor.read_line() {
                println!();

                if input.is_empty() || input == "\n" {
                    continue;
                }

                if input == "exit" {
                    break;
                }

                execute_pyql_query(
                    input.to_owned(),
                    arguments,
                    &mut global_env,
                    &provider,
                    &mut reporter,
                );

                global_env.clear_session();
            }
        }
        return;
    }

    let mut input = String::new();
    loop {
        let stdin = io::stdin();

        // Render Prompt only if input is received from terminal
        if stdin.is_terminal() {
            print!("pyql > ");
        }

        std::io::Write::flush(&mut std::io::stdout()).expect("flush failed!");
        match std::io::stdin().read_line(&mut input) {
            Ok(buffer_length) => {
                if buffer_length == 0 {
                    break;
                }
            }
            Err(error) => {
                reporter.report_diagnostic(&input, Diagnostic::error(&format!("{}", error)));
            }
        }

        let stdin_input = input.trim();
        if stdin_input.is_empty() || stdin_input == "\n" {
            continue;
        }

        if stdin_input == "exit" {
            println!("Goodbye!");
            break;
        }

        execute_pyql_query(
            stdin_input.to_owned(),
            arguments,
            &mut global_env,
            &provider,
            &mut reporter,
        );

        input.clear();
        global_env.clear_session();
    }
}

#[allow(clippy::borrowed_box)]
fn execute_pyql_query(
    query: String,
    arguments: &Arguments,
    env: &mut Environment,
    provider: &Box<dyn DataProvider>,
    reporter: &mut DiagnosticReporter,
) {
    let front_start = std::time::Instant::now();
    let tokenizer_result = Tokenizer::tokenize(query.clone());
    if tokenizer_result.is_err() {
        let diagnostic = tokenizer_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return;
    }

    let tokens = tokenizer_result.ok().unwrap();
    if tokens.is_empty() {
        return;
    }

    let parser_result = parser::parse_gql(tokens, env);
    if parser_result.is_err() {
        let diagnostic = parser_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return;
    }

    let query_node = parser_result.ok().unwrap();
    let front_duration = front_start.elapsed();

    let engine_start = std::time::Instant::now();
    let evaluation_result = engine::evaluate(env, provider, query_node);
    let engine_duration = engine_start.elapsed();

    // Report Runtime exceptions if they exists
    if evaluation_result.is_err() {
        let exception = Diagnostic::exception(&evaluation_result.err().unwrap());
        reporter.report_diagnostic(&query, exception);
        return;
    }

    // Render the result only if they are selected groups not any other statement
    let printer: Box<dyn OutputPrinter> = match arguments.output_format {
        OutputFormat::Render => {
            Box::new(TablePrinter::new(arguments.pagination, arguments.page_size))
        }
        OutputFormat::JSON => Box::new(JSONPrinter),
        OutputFormat::CSV => Box::new(CSVPrinter),
    };

    // Render the result only if they are selected groups not any other statement
    let evaluations_results = evaluation_result.ok().unwrap();
    for evaluation_result in evaluations_results {
        let mut rows_count = 0;
        if let SelectedGroups(mut groups) = evaluation_result {
            rows_count += groups.len();
            printer.print(&mut groups);
        }

        if arguments.analysis {
            let total_time = front_duration + engine_duration;
            println!(
                "{} row in set (total: {:?}, front: {:?}, engine: {:?})",
                rows_count, total_time, front_duration, engine_duration
            );
        }
    }
}
