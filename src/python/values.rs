use std::any::Any;
use std::cmp::Ordering;

use gitql_ast::types::base::DataType;
use gitql_core::values::base::Value;
use rustpython_parser::ast::StmtFunctionDef;

use super::types::PyFunctionType;

#[derive(Clone)]
pub struct PyFunctionValue {
    pub function: StmtFunctionDef,
}

impl Value for PyFunctionValue {
    fn literal(&self) -> String {
        let function = &self.function;
        let arguments: Vec<String> = function
            .args
            .args
            .iter()
            .map(|arg| arg.as_arg().arg.to_string())
            .collect();
        let arguments_joined = arguments.join(", ");
        format!("def {}({}):", function.name, arguments_joined)
    }

    fn equals(&self, other: &Box<dyn Value>) -> bool {
        if let Some(other_value) = other.as_any().downcast_ref::<PyFunctionValue>() {
            return self.function.eq(&other_value.function);
        }
        false
    }

    fn compare(&self, _other: &Box<dyn Value>) -> Option<Ordering> {
        None
    }

    fn data_type(&self) -> Box<dyn DataType> {
        Box::new(PyFunctionType)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
