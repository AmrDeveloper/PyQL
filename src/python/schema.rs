use std::collections::HashMap;

use gitql_ast::types::base::DataType;
use gitql_ast::types::integer::IntType;
use gitql_ast::types::text::TextType;

use super::types::PyFunctionType;

pub fn pyql_tables_fields_types() -> HashMap<&'static str, Box<dyn DataType>> {
    let mut map: HashMap<&str, Box<dyn DataType>> = HashMap::new();

    // Functions Table
    map.insert("function_name", Box::new(TextType));
    map.insert("arguments_count", Box::new(IntType));
    map.insert("function", Box::new(PyFunctionType));
    map.insert("file_name", Box::new(IntType));

    map
}

pub fn pyql_tables_fields_names() -> HashMap<&'static str, Vec<&'static str>> {
    let mut map = HashMap::new();
    map.insert(
        "functions",
        vec!["function_name", "arguments_count", "function", "file_name"],
    );
    map
}
