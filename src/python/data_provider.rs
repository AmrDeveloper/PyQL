use gitql_core::object::Row;
use gitql_core::values::base::Value;
use gitql_core::values::integer::IntValue;
use gitql_core::values::null::NullValue;
use gitql_core::values::text::TextValue;
use gitql_engine::data_provider::DataProvider;
use rustpython_parser::ast::Stmt;
use rustpython_parser::ast::StmtFunctionDef;

use super::values::PyFunctionValue;

pub struct PythonDataProvider {
    pub paths: Vec<String>,
    pub modules: Vec<Vec<Stmt>>,
}

impl PythonDataProvider {
    pub fn new(paths: Vec<String>, modules: Vec<Vec<Stmt>>) -> Self {
        Self { paths, modules }
    }
}

impl DataProvider for PythonDataProvider {
    fn provide(&self, _table: &str, selected_columns: &[String]) -> Result<Vec<Row>, String> {
        let mut rows: Vec<Row> = vec![];
        for (path_index, path) in self.paths.iter().enumerate() {
            let module = &self.modules[path_index];
            let mut selected_rows = select_python_functions(path, module, selected_columns)?;
            rows.append(&mut selected_rows);
        }
        Ok(rows)
    }
}

fn select_python_functions(
    path: &str,
    module_statement: &Vec<Stmt>,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = vec![];
    let row_width = selected_columns.len();

    let mut python_functions: Vec<StmtFunctionDef> = vec![];
    for statement in module_statement {
        if let Some(function) = statement.as_function_def_stmt() {
            python_functions.push(function.clone());
            continue;
        }

        if let Some(class) = statement.as_class_def_stmt() {
            for class_statement in class.body.iter() {
                if let Some(function) = class_statement.as_function_def_stmt() {
                    python_functions.push(function.clone());
                    continue;
                }
            }
        }
    }

    for python_function in python_functions.iter() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(row_width);
        for column_name in selected_columns {
            if column_name == "function_name" {
                let value = python_function.name.to_string();
                values.push(Box::new(TextValue { value }));
                continue;
            }

            if column_name == "arguments_count" {
                let value = python_function.args.args.len() as i64;
                values.push(Box::new(IntValue { value }));
                continue;
            }

            if column_name == "file_name" {
                let value = path.to_string();
                values.push(Box::new(TextValue { value }));
                continue;
            }

            if column_name == "function" {
                let function = python_function.clone();
                values.push(Box::new(PyFunctionValue { function }));
                continue;
            }

            values.push(Box::new(NullValue));
        }
        rows.push(Row { values });
    }

    Ok(rows)
}
