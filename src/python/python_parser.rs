use std::fs;

use rustpython_parser::ast;
use rustpython_parser::ast::Stmt;
use rustpython_parser::Parse;

pub(crate) fn parse_python_files(paths: &[String]) -> Result<Vec<Vec<Stmt>>, String> {
    let mut modules: Vec<Vec<Stmt>> = Vec::with_capacity(paths.len());

    for path in paths {
        let reading_py_file_result = fs::read_to_string(path);
        if let Err(reading_error) = reading_py_file_result {
            return Err(reading_error.to_string());
        }

        let py_source_code = reading_py_file_result.ok().unwrap();
        let parser_result = ast::Suite::parse(&py_source_code, path);
        if let Err(parser_error) = parser_result {
            return Err(parser_error.error.to_string());
        }

        let statements = parser_result.ok().unwrap();
        modules.push(statements);
    }

    Ok(modules)
}
