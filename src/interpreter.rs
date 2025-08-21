use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::fmt;

use crate::data_format::DataFormat;
use crate::parser::{
    BinaryOp, Command, Enum, Expr, Init, InterpolationPart, RigFile, Statement, Status,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    EnumVariant {
        enum_name: String,
        variant_name: String,
        value: u32,
    },
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{i}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::EnumVariant {
                enum_name,
                variant_name,
                value,
            } => {
                write!(f, "{enum_name}::{variant_name}({value})")
            }
            Value::Unit => write!(f, "()"),
        }
    }
}

impl Value {
    pub fn to_interpolation_string(&self) -> String {
        match self {
            Value::Integer(i) => format!("{i:02X}"),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            Value::EnumVariant { value, .. } => format!("{value:02X}"),
            Value::Unit => String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Env {
    variables: HashMap<String, Value>,
    parent: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Env) -> Self {
        Env {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct InterpreterContext {
    pub environment: Env,
    pub enums: HashMap<String, HashMap<String, u32>>,
    pub output: Vec<String>,
}

impl InterpreterContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_enum(&mut self, enum_def: &Enum) {
        let mut variants = HashMap::new();
        for variant in &enum_def.variants {
            variants.insert(variant.name.clone(), variant.value);
        }
        self.enums.insert(enum_def.name.clone(), variants);
    }

    pub fn get_enum_variant(&self, enum_name: &str, variant_name: &str) -> Option<u32> {
        self.enums.get(enum_name)?.get(variant_name).copied()
    }
}

pub trait BuiltinFunction {
    fn call(&self, args: &[Value], context: &mut InterpreterContext) -> Result<Value>;
}

pub struct WriteFunction;

impl BuiltinFunction for WriteFunction {
    fn call(&self, args: &[Value], context: &mut InterpreterContext) -> Result<Value> {
        if args.len() != 1 {
            return Err(anyhow!(
                "write() expects exactly 1 argument, got {}",
                args.len()
            ));
        }

        let output = args[0].to_string();
        context.output.push(format!("WRITE: {output}"));
        Ok(Value::Unit)
    }
}

pub struct ReadFunction;

impl BuiltinFunction for ReadFunction {
    fn call(&self, args: &[Value], context: &mut InterpreterContext) -> Result<Value> {
        if args.len() != 1 {
            return Err(anyhow!(
                "read() expects exactly 1 argument, got {}",
                args.len()
            ));
        }

        let expected = args[0].to_string();
        context.output.push(format!("READ: {expected}"));
        Ok(Value::Unit)
    }
}

pub struct FormatFunction;

impl BuiltinFunction for FormatFunction {
    fn call(&self, args: &[Value], _context: &mut InterpreterContext) -> Result<Value> {
        if args.is_empty() {
            return Err(anyhow!("format() expects at least 1 argument"));
        }

        let formatted = args[0].to_string();
        Ok(Value::String(formatted))
    }
}

pub struct Interpreter {
    builtins: HashMap<String, Box<dyn BuiltinFunction>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut builtins: HashMap<String, Box<dyn BuiltinFunction>> = HashMap::new();
        builtins.insert("write".to_string(), Box::new(WriteFunction));
        builtins.insert("read".to_string(), Box::new(ReadFunction));
        builtins.insert("format".to_string(), Box::new(FormatFunction));

        Interpreter { builtins }
    }

    pub fn execute_rig_file(&self, rig_file: &RigFile) -> Result<InterpreterContext> {
        let mut context = InterpreterContext::new();

        for (id, expr) in &rig_file.settings.settings {
            let value = self.evaluate_expression(expr, &mut context)?;
            context.environment.set(id.to_string(), value);
        }

        for enum_def in &rig_file.impl_block.enums {
            context.register_enum(enum_def);
        }

        if let Some(init) = &rig_file.impl_block.init {
            self.execute_init(init, &mut context)?;
        }

        Ok(context)
    }

    pub fn execute_command(
        &self,
        command: &Command,
        args: &[Value],
        context: &mut InterpreterContext,
    ) -> Result<()> {
        let mut local_env = Env::with_parent(context.environment.clone());

        if args.len() != command.parameters.len() {
            return Err(anyhow!(
                "Command '{}' expects {} arguments, got {}",
                command.name,
                command.parameters.len(),
                args.len()
            ));
        }

        for (param, arg) in command.parameters.iter().zip(args.iter()) {
            local_env.set(param.name.clone(), arg.clone());
        }

        let old_env = std::mem::replace(&mut context.environment, local_env);

        for statement in &command.statements {
            self.execute_statement(statement, context)?;
        }

        context.environment = old_env;

        Ok(())
    }

    pub fn execute_init(&self, init: &Init, context: &mut InterpreterContext) -> Result<()> {
        for statement in &init.statements {
            self.execute_statement(statement, context)?;
        }
        Ok(())
    }

    pub fn execute_status(&self, status: &Status, context: &mut InterpreterContext) -> Result<()> {
        for statement in &status.statements {
            self.execute_statement(statement, context)?;
        }
        Ok(())
    }

    pub fn execute_statement(
        &self,
        statement: &Statement,
        context: &mut InterpreterContext,
    ) -> Result<()> {
        match statement {
            Statement::Assign(id, expr) => {
                let value = self.evaluate_expression(expr, context)?;
                context.environment.set(id.to_string(), value);
            }
            Statement::FunctionCall { name, args } => {
                let arg_values: Result<Vec<_>> = args
                    .iter()
                    .map(|arg| self.evaluate_expression(arg, context))
                    .collect();
                let arg_values = arg_values?;

                if let Some(builtin) = self.builtins.get(name) {
                    builtin.call(&arg_values, context)?;
                } else {
                    return Err(anyhow!("Unknown function: {}", name));
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_value = self.evaluate_expression(condition, context)?;
                match condition_value {
                    Value::Boolean(true) => {
                        for stmt in then_body {
                            self.execute_statement(stmt, context)?;
                        }
                    }
                    Value::Boolean(false) => {
                        if let Some(else_stmts) = else_body {
                            for stmt in else_stmts {
                                self.execute_statement(stmt, context)?;
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!(
                            "If condition must be a boolean, got: {:?}",
                            condition_value
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn evaluate_expression(
        &self,
        expr: &Expr,
        context: &mut InterpreterContext,
    ) -> Result<Value> {
        match expr {
            Expr::Integer(i) => Ok(Value::Integer(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Identifier(id) => context
                .environment
                .get(id.as_str())
                .ok_or_else(|| anyhow!("Undefined variable: {}", id.as_str())),
            Expr::QualifiedIdentifier(scope, id) => {
                if let Some(value) = context.get_enum_variant(scope.as_str(), id.as_str()) {
                    Ok(Value::EnumVariant {
                        enum_name: scope.to_string(),
                        variant_name: id.to_string(),
                        value,
                    })
                } else {
                    Err(anyhow!(
                        "Unknown qualified identifier: {}::{}",
                        scope.as_str(),
                        id.as_str()
                    ))
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(left, context)?;
                let right_val = self.evaluate_expression(right, context)?;
                Self::apply_binary_op(&left_val, op, &right_val)
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                let object_val = self.evaluate_expression(object, context)?;
                let arg_values: Result<Vec<_>> = args
                    .iter()
                    .map(|arg| self.evaluate_expression(arg, context))
                    .collect();
                let arg_values = arg_values?;

                self.call_method(&object_val, method, &arg_values, context)
            }
            Expr::StringInterpolation { parts } => {
                self.process_parsed_string_interpolation(parts, context)
            }
        }
    }

    fn apply_binary_op(left: &Value, op: &BinaryOp, right: &Value) -> Result<Value> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => match op {
                BinaryOp::Add => Ok(Value::Integer(a + b)),
                BinaryOp::Subtract => Ok(Value::Integer(a - b)),
                BinaryOp::Multiply => Ok(Value::Integer(a * b)),
                BinaryOp::Divide => Ok(Value::Integer(a / b)),
                BinaryOp::Modulo => Ok(Value::Integer(a % b)),
                BinaryOp::Equal => Ok(Value::Boolean(a == b)),
                BinaryOp::NotEqual => Ok(Value::Boolean(a != b)),
                BinaryOp::Less => Ok(Value::Boolean(a < b)),
                BinaryOp::LessEqual => Ok(Value::Boolean(a <= b)),
                BinaryOp::Greater => Ok(Value::Boolean(a > b)),
                BinaryOp::GreaterEqual => Ok(Value::Boolean(a >= b)),
                BinaryOp::And => Ok(Value::Boolean(*a != 0 && *b != 0)),
                BinaryOp::Or => Ok(Value::Boolean(*a != 0 || *b != 0)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                BinaryOp::Add => Ok(Value::Float(a + b)),
                BinaryOp::Subtract => Ok(Value::Float(a - b)),
                BinaryOp::Multiply => Ok(Value::Float(a * b)),
                BinaryOp::Divide => Ok(Value::Float(a / b)),
                BinaryOp::Modulo => Ok(Value::Float(a % b)),
                BinaryOp::Equal => Ok(Value::Boolean((a - b).abs() < f64::EPSILON)),
                BinaryOp::NotEqual => Ok(Value::Boolean((a - b).abs() >= f64::EPSILON)),
                BinaryOp::Less => Ok(Value::Boolean(a < b)),
                BinaryOp::LessEqual => Ok(Value::Boolean(a <= b)),
                BinaryOp::Greater => Ok(Value::Boolean(a > b)),
                BinaryOp::GreaterEqual => Ok(Value::Boolean(a >= b)),
                BinaryOp::And => Ok(Value::Boolean(*a != 0.0 && *b != 0.0)),
                BinaryOp::Or => Ok(Value::Boolean(*a != 0.0 || *b != 0.0)),
            },
            (Value::Integer(a), Value::Float(b)) => {
                Self::apply_binary_op(&Value::Float(*a as f64), op, &Value::Float(*b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                Self::apply_binary_op(&Value::Float(*a), op, &Value::Float(*b as f64))
            }
            (Value::String(a), Value::String(b)) => match op {
                BinaryOp::Add => Ok(Value::String(format!("{a}{b}"))),
                BinaryOp::Equal => Ok(Value::Boolean(a == b)),
                BinaryOp::NotEqual => Ok(Value::Boolean(a != b)),
                _ => Err(anyhow!("Invalid operation {:?} for strings", op)),
            },
            (Value::Boolean(a), Value::Boolean(b)) => match op {
                BinaryOp::Equal => Ok(Value::Boolean(a == b)),
                BinaryOp::NotEqual => Ok(Value::Boolean(a != b)),
                BinaryOp::And => Ok(Value::Boolean(*a && *b)),
                BinaryOp::Or => Ok(Value::Boolean(*a || *b)),
                _ => Err(anyhow!("Invalid operation {:?} for booleans", op)),
            },
            _ => Err(anyhow!(
                "Type mismatch in binary operation: {:?} {:?} {:?}",
                left,
                op,
                right
            )),
        }
    }

    fn process_parsed_string_interpolation(
        &self,
        parts: &[InterpolationPart],
        context: &mut InterpreterContext,
    ) -> Result<Value> {
        let mut result = Vec::new();

        for part in parts {
            match part {
                InterpolationPart::Literal(bytes) => {
                    result.extend_from_slice(bytes);
                }
                InterpolationPart::Variable {
                    name,
                    format,
                    length,
                } => {
                    let interpolated = self.interpolate_parsed_variable(
                        name,
                        format.as_deref(),
                        *length,
                        context,
                    )?;
                    result.extend_from_slice(&interpolated);
                }
            }
        }

        let hex_string = result
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>();

        Ok(Value::String(hex_string))
    }

    fn interpolate_parsed_variable(
        &self,
        name: &str,
        format: Option<&str>,
        length: usize,
        context: &mut InterpreterContext,
    ) -> Result<Vec<u8>> {
        let value = context
            .environment
            .get(name)
            .ok_or_else(|| anyhow!("Undefined variable: {}", name))?;

        match format {
            None => {
                // {name:length} - use default format (int_lu) with specified length
                match value {
                    Value::Integer(i) => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(i as i32, length)?;
                        Ok(bytes)
                    }
                    Value::String(s) => {
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.resize(length, 0);
                        Ok(bytes)
                    }
                    Value::EnumVariant { value, .. } => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(value as i32, length)?;
                        Ok(bytes)
                    }
                    _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
                }
            }
            Some(format_str) => {
                // {name:format} or {name:format:length} - use specified format
                let format = DataFormat::try_from(format_str)
                    .map_err(|_| anyhow!("Invalid format: {}", format_str))?;

                match value {
                    Value::Integer(i) => {
                        let bytes = format.encode(i as i32, length)?;
                        Ok(bytes)
                    }
                    Value::String(s) => {
                        if format == DataFormat::Text {
                            let bytes = format.encode(s.parse::<i32>().unwrap_or(0), length)?;
                            Ok(bytes)
                        } else {
                            let mut bytes = s.as_bytes().to_vec();
                            bytes.resize(length, 0);
                            Ok(bytes)
                        }
                    }
                    Value::EnumVariant { value, .. } => {
                        let bytes = format.encode(value as i32, length)?;
                        Ok(bytes)
                    }
                    _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
                }
            }
        }
    }

    /// Old interpolate_variable method - keeping for reference
    fn _old_interpolate_variable(
        &self,
        var_spec: &str,
        context: &mut InterpreterContext,
    ) -> Result<Vec<u8>> {
        let parts: Vec<&str> = var_spec.split(':').collect();

        match parts.len() {
            1 => {
                let var_name = parts[0];
                let value = context
                    .environment
                    .get(var_name)
                    .ok_or_else(|| anyhow!("Undefined variable: {}", var_name))?;

                match value {
                    Value::Integer(i) => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(i as i32, 4)?;
                        Ok(bytes)
                    }
                    Value::String(s) => Ok(s.as_bytes().to_vec()),
                    Value::EnumVariant { value, .. } => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(value as i32, 1)?;
                        Ok(bytes)
                    }
                    _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
                }
            }
            2 => {
                let var_name = parts[0];
                let length = parts[1]
                    .parse::<usize>()
                    .map_err(|_| anyhow!("Invalid length: {}", parts[1]))?;

                let value = context
                    .environment
                    .get(var_name)
                    .ok_or_else(|| anyhow!("Undefined variable: {}", var_name))?;

                match value {
                    Value::Integer(i) => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(i as i32, length)?;
                        Ok(bytes)
                    }
                    Value::String(s) => {
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.resize(length, 0);
                        Ok(bytes)
                    }
                    Value::EnumVariant { value, .. } => {
                        let format = DataFormat::IntLu;
                        let bytes = format.encode(value as i32, length)?;
                        Ok(bytes)
                    }
                    _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
                }
            }
            3 => {
                let var_name = parts[0];
                let format_str = parts[1];
                let length = parts[2]
                    .parse::<usize>()
                    .map_err(|_| anyhow!("Invalid length: {}", parts[2]))?;

                let format = DataFormat::try_from(format_str)
                    .map_err(|_| anyhow!("Invalid format: {}", format_str))?;

                let value = context
                    .environment
                    .get(var_name)
                    .ok_or_else(|| anyhow!("Undefined variable: {}", var_name))?;

                match value {
                    Value::Integer(i) => {
                        let bytes = format.encode(i as i32, length)?;
                        Ok(bytes)
                    }
                    Value::String(s) => {
                        if format == DataFormat::Text {
                            let bytes = format.encode(s.parse::<i32>().unwrap_or(0), length)?;
                            Ok(bytes)
                        } else {
                            let mut bytes = s.as_bytes().to_vec();
                            bytes.resize(length, 0);
                            Ok(bytes)
                        }
                    }
                    Value::EnumVariant { value, .. } => {
                        let bytes = format.encode(value as i32, length)?;
                        Ok(bytes)
                    }
                    _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
                }
            }
            _ => Err(anyhow!("Invalid variable specification: {}", var_spec)),
        }
    }

    fn call_method(
        &self,
        object: &Value,
        method: &str,
        args: &[Value],
        _context: &mut InterpreterContext,
    ) -> Result<Value> {
        match method {
            "format" => match object {
                Value::Integer(i) => {
                    if args.len() >= 2 {
                        Ok(Value::String(format!("{i:0width$X}", width = 2)))
                    } else {
                        Ok(Value::String(format!("{i:02X}")))
                    }
                }
                _ => Ok(Value::String(object.to_string())),
            },
            _ => Err(anyhow!("Unknown method: {}", method)),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{EnumVariant, Id, parse};

    #[test]
    fn test_basic_expression_evaluation() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        let expr = Expr::Integer(42);
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::Integer(42));

        let expr = Expr::Float(3.5);
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::Float(3.5));

        let expr = Expr::String("hello".to_string());
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_binary_operations() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Integer(5)),
        };
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::Integer(15));

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Greater,
            right: Box::new(Expr::Integer(5)),
        };
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_variable_assignment_and_lookup() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        let statement = Statement::Assign(Id::new("x"), Expr::Integer(42));
        interpreter.execute_statement(&statement, &mut context)?;

        let expr = Expr::Identifier(Id::new("x"));
        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        assert_eq!(result, Value::Integer(42));
        Ok(())
    }

    #[test]
    fn test_function_call() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        let statement = Statement::FunctionCall {
            name: "write".to_string(),
            args: vec![Expr::String("test".to_string())],
        };
        interpreter.execute_statement(&statement, &mut context)?;

        assert_eq!(context.output.len(), 1);
        assert_eq!(context.output[0], "WRITE: test");
        Ok(())
    }

    #[test]
    fn test_string_interpolation() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        context
            .environment
            .set("freq".to_string(), Value::Integer(14500000));
        context
            .environment
            .set("vfo".to_string(), Value::String("A".to_string()));

        let expr = Expr::StringInterpolation {
            parts: vec![
                InterpolationPart::Literal(vec![0xFE, 0xFE, 0x94, 0xE0]),
                InterpolationPart::Literal(vec![0x25]),
                InterpolationPart::Variable {
                    name: "vfo".to_string(),
                    format: None,
                    length: 1,
                },
                InterpolationPart::Variable {
                    name: "freq".to_string(),
                    format: Some("int_lu".to_string()),
                    length: 4,
                },
                InterpolationPart::Literal(vec![0xFD]),
            ],
        };

        let result = interpreter.evaluate_expression(&expr, &mut context)?;
        // FEFE94E0 (literal hex) + 25 (literal hex) + 41 (vfo="A" as ASCII) + A040DD00 (freq=14500000 in int_lu:4) + FD (literal hex)
        assert_eq!(result, Value::String("FEFE94E02541A040DD00FD".to_string()));
        Ok(())
    }

    #[test]
    fn test_if_statement() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        context.environment.set("x".to_string(), Value::Integer(10));

        let statement = Statement::If {
            condition: Expr::BinaryOp {
                left: Box::new(Expr::Identifier(Id::new("x"))),
                op: BinaryOp::Greater,
                right: Box::new(Expr::Integer(5)),
            },
            then_body: vec![Statement::FunctionCall {
                name: "write".to_string(),
                args: vec![Expr::String("condition true".to_string())],
            }],
            else_body: Some(vec![Statement::FunctionCall {
                name: "write".to_string(),
                args: vec![Expr::String("condition false".to_string())],
            }]),
        };

        interpreter.execute_statement(&statement, &mut context)?;

        assert_eq!(context.output.len(), 1);
        assert_eq!(context.output[0], "WRITE: condition true");
        Ok(())
    }

    #[test]
    fn test_enum_handling() -> Result<()> {
        let interpreter = Interpreter::new();
        let mut context = InterpreterContext::new();

        let enum_def = Enum {
            name: "Vfo".to_string(),
            variants: vec![
                EnumVariant {
                    name: "A".to_string(),
                    value: 0,
                },
                EnumVariant {
                    name: "B".to_string(),
                    value: 1,
                },
            ],
        };
        context.register_enum(&enum_def);

        let expr = Expr::QualifiedIdentifier(Id::new("Vfo"), Id::new("A"));
        let result = interpreter.evaluate_expression(&expr, &mut context)?;

        match result {
            Value::EnumVariant {
                enum_name,
                variant_name,
                value,
            } => {
                assert_eq!(enum_name, "Vfo");
                assert_eq!(variant_name, "A");
                assert_eq!(value, 0);
                Ok(())
            }
            _ => Err(anyhow!("Expected enum variant")),
        }
    }

    #[test]
    fn test_simple_rig_file_execution() -> Result<()> {
        let dsl_source = r#"
            version = 1;
            baudrate = 9600;
            impl Transceiver for TestRig {
                enum Vfo {
                    A = 0,
                    B = 1,
                }
                init {
                    write("initialization");
                }
                fn set_freq(int freq, Vfo vfo) {
                    command = "FEFE94E0.25.{vfo}.{freq}.FD";
                    write(command);
                }
            }
        "#;

        let rig_file = parse(dsl_source).unwrap();
        let interpreter = Interpreter::new();
        let mut context = interpreter.execute_rig_file(&rig_file).unwrap();

        assert_eq!(context.environment.get("version"), Some(Value::Integer(1)));
        assert_eq!(
            context.environment.get("baudrate"),
            Some(Value::Integer(9600))
        );

        assert_eq!(context.get_enum_variant("Vfo", "A"), Some(0));
        assert_eq!(context.get_enum_variant("Vfo", "B"), Some(1));

        assert!(context.output.len() >= 2);
        assert_eq!(context.output[0], "WRITE: initialization");
        assert_eq!(context.output[1], "DELAY: 100ms");

        let command = &rig_file.impl_block.commands[0];
        let args = vec![
            Value::Integer(14500000),
            Value::EnumVariant {
                enum_name: "Vfo".to_string(),
                variant_name: "A".to_string(),
                value: 0,
            },
        ];
        interpreter.execute_command(command, &args, &mut context)?;

        let last_output = context.output.last().unwrap();
        assert!(last_output.contains("WRITE: FEFE94E0.25.00.DD4DA0.FD"));
        Ok(())
    }
}
