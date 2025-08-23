use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::fmt;

use crate::data_format::DataFormat;
use crate::parser::{BinaryOp, Expr, InterpolationPart, RigFile, Statement};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Bytes(Vec<u8>),
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
            Value::Bytes(b) => {
                if let Ok(string) = String::from_utf8(b.clone()) {
                    write!(f, "{string}")
                } else {
                    write!(f, "{b:?}")
                }
            }
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

#[derive(Debug, Clone, Default)]
pub struct Env {
    variables: HashMap<String, Value>,
    parent: Option<Box<Env>>,
    pub output: Vec<String>,
    enums: HashMap<String, HashMap<String, u32>>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Env) -> Self {
        Env {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
            output: Vec::new(),
            enums: HashMap::new(),
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

    pub fn get_enum_variant(&self, enum_name: &str, variant_name: &str) -> Option<u32> {
        self.enums
            .get(enum_name)
            .and_then(|variants| variants.get(variant_name).copied())
            .or_else(|| {
                self.parent
                    .as_ref()
                    .and_then(|parent| parent.get_enum_variant(enum_name, variant_name))
            })
    }

    pub fn register_enum(&mut self, enum_def: &crate::parser::Enum) {
        self.enums.insert(
            enum_def.name.clone(),
            enum_def.variants.clone().into_iter().collect(),
        );
    }
}

pub trait Builtins {
    fn call(&self, name: &str, args: &[Value], env: &mut Env) -> Result<Value>;
}

struct DummyBuiltins;

impl Builtins for DummyBuiltins {
    fn call(&self, name: &str, args: &[Value], env: &mut Env) -> Result<Value> {
        match name {
            "read" => {
                if args.len() != 1 {
                    return Err(anyhow!(
                        "read() expects exactly 1 argument, got {}",
                        args.len()
                    ));
                }

                let expected = args[0].to_string();
                env.output.push(format!("READ: {expected}"));
                Ok(Value::Unit)
            }
            "write" => {
                if args.len() != 1 {
                    return Err(anyhow!(
                        "write() expects exactly 1 argument, got {}",
                        args.len()
                    ));
                }

                let output = args[0].to_string();
                env.output.push(format!("WRITE: {output}"));
                Ok(Value::Unit)
            }
            _ => Err(anyhow!("Unkown function {name}")),
        }
    }
}

pub struct Interpreter {
    rig_file: RigFile,
}

impl Interpreter {
    pub fn new(rig_file: RigFile) -> Self {
        Self { rig_file }
    }

    pub fn create_env(&self) -> Result<Env> {
        let mut env = Env::new();

        for (id, expr) in &self.rig_file.settings.settings {
            let value = self.evaluate_expression(expr, &mut env)?;
            env.set(id.to_string(), value);
        }

        for enum_def in &self.rig_file.impl_block.enums {
            env.register_enum(enum_def);
        }

        Ok(env)
    }

    pub fn execute_command(
        &self,
        name: &str,
        args: &[Value],
        builtins: &impl Builtins,
        env: &mut Env,
    ) -> Result<()> {
        let command = self
            .rig_file
            .impl_block
            .commands
            .get(name)
            .context("Unknown command name")?;
        if args.len() != command.parameters.len() {
            return Err(anyhow!(
                "Command '{}' expects {} arguments, got {}",
                command.name,
                command.parameters.len(),
                args.len()
            ));
        }

        let mut local_env = Env::with_parent(env.clone());
        for (param, arg) in command.parameters.iter().zip(args.iter()) {
            local_env.set(param.name.clone(), arg.clone());
        }

        for statement in &command.statements {
            self.execute_statement(statement, builtins, &mut local_env)?;
        }

        env.output.extend(local_env.output);

        Ok(())
    }

    pub fn execute_init(&self, builtins: &impl Builtins, env: &mut Env) -> Result<()> {
        if let Some(init) = &self.rig_file.impl_block.init {
            for statement in &init.statements {
                self.execute_statement(statement, builtins, env)?;
            }
        }
        Ok(())
    }

    pub fn execute_status(&self, builtins: &impl Builtins, env: &mut Env) -> Result<()> {
        if let Some(status) = &self.rig_file.impl_block.status {
            for statement in &status.statements {
                self.execute_statement(statement, builtins, env)?;
            }
        }
        Ok(())
    }

    pub fn execute_statement(
        &self,
        statement: &Statement,
        builtins: &impl Builtins,
        env: &mut Env,
    ) -> Result<()> {
        match statement {
            Statement::Assign(id, expr) => {
                let value = self.evaluate_expression(expr, env)?;
                env.set(id.to_string(), value);
            }
            Statement::FunctionCall { name, args } => {
                let arg_values = args
                    .iter()
                    .map(|arg| self.evaluate_expression(arg, env))
                    .collect::<Result<Vec<_>>>()?;

                let _ = builtins.call(name, &arg_values, env)?;
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_value = self.evaluate_expression(condition, env)?;
                match condition_value {
                    Value::Boolean(true) => {
                        for stmt in then_body {
                            self.execute_statement(stmt, builtins, env)?;
                        }
                    }
                    Value::Boolean(false) => {
                        if let Some(else_stmts) = else_body {
                            for stmt in else_stmts {
                                self.execute_statement(stmt, builtins, env)?;
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

    pub fn evaluate_expression(&self, expr: &Expr, env: &mut Env) -> Result<Value> {
        match expr {
            Expr::Integer(i) => Ok(Value::Integer(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::Bytes(s.as_bytes().to_vec())),
            Expr::Identifier(id) => env
                .get(id.as_str())
                .ok_or_else(|| anyhow!("Undefined variable: {}", id.as_str())),
            Expr::QualifiedIdentifier(scope, id) => {
                if let Some(value) = env.get_enum_variant(scope.as_str(), id.as_str()) {
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
                let left_val = self.evaluate_expression(left, env)?;
                let right_val = self.evaluate_expression(right, env)?;
                Self::apply_binary_op(&left_val, op, &right_val)
            }
            Expr::StringInterpolation { parts } => {
                self.process_parsed_string_interpolation(parts, env)
            }
        }
    }

    fn apply_binary_op(left: &Value, op: &BinaryOp, right: &Value) -> Result<Value> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => match op {
                BinaryOp::Add => Ok(Value::Integer(a + b)),
                BinaryOp::Subtract => Ok(Value::Integer(a - b)),
                BinaryOp::Multiply => Ok(Value::Integer(a * b)),
                BinaryOp::Divide => {
                    if *b == 0 {
                        Err(anyhow!("Division by zero"))
                    } else {
                        Ok(Value::Integer(a / b))
                    }
                }
                BinaryOp::Modulo => {
                    if *b == 0 {
                        Err(anyhow!("Modulo by zero"))
                    } else {
                        Ok(Value::Integer(a % b))
                    }
                }
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
                BinaryOp::And | BinaryOp::Or => {
                    Err(anyhow!("Binary operator is not supported in floats"))
                }
            },
            (Value::Integer(a), Value::Float(b)) => {
                Self::apply_binary_op(&Value::Float(*a as f64), op, &Value::Float(*b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                Self::apply_binary_op(&Value::Float(*a), op, &Value::Float(*b as f64))
            }
            (Value::Bytes(a), Value::Bytes(b)) => match op {
                BinaryOp::Add => {
                    let mut result = a.clone();
                    result.extend_from_slice(b);
                    Ok(Value::Bytes(result))
                }
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
        env: &mut Env,
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
                    let interpolated =
                        self.interpolate_parsed_variable(name, format.as_deref(), *length, env)?;
                    result.extend_from_slice(&interpolated);
                }
            }
        }

        Ok(Value::Bytes(result))
    }

    fn interpolate_parsed_variable(
        &self,
        name: &str,
        format: Option<&str>,
        length: usize,
        env: &mut Env,
    ) -> Result<Vec<u8>> {
        let value = env
            .get(name)
            .ok_or_else(|| anyhow!("Undefined variable: {}", name))?;

        match format {
            None => match value {
                Value::Integer(i) => {
                    let format = DataFormat::IntLu;
                    let bytes = format.encode(i as i32, length)?;
                    Ok(bytes)
                }
                Value::EnumVariant { value, .. } => {
                    let format = DataFormat::IntLu;
                    let bytes = format.encode(value as i32, length)?;
                    Ok(bytes)
                }
                _ => Err(anyhow!("Cannot interpolate value type: {:?}", value)),
            },
            Some(format_str) => {
                let format = DataFormat::try_from(format_str)
                    .map_err(|_| anyhow!("Invalid format: {}", format_str))?;

                match value {
                    Value::Integer(i) => {
                        let bytes = format.encode(i as i32, length)?;
                        Ok(bytes)
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
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Id, parse};
    use std::collections::BTreeMap;

    #[test]
    fn test_basic_expression_evaluation() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::Integer(42);
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Integer(42));

        let expr = Expr::Float(3.5);
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Float(3.5));

        let expr = Expr::String("hello".to_string());
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Bytes("hello".as_bytes().to_vec()));
        Ok(())
    }

    #[test]
    fn test_binary_operations() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Integer(5)),
        };
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Integer(15));

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Greater,
            right: Box::new(Expr::Integer(5)),
        };
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_variable_assignment_and_lookup() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let statement = Statement::Assign(Id::new("x"), Expr::Integer(42));
        interpreter.execute_statement(&statement, &DummyBuiltins, &mut env)?;

        let expr = Expr::Identifier(Id::new("x"));
        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Integer(42));
        Ok(())
    }

    #[test]
    fn test_function_call() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let statement = Statement::FunctionCall {
            name: "write".to_string(),
            args: vec![Expr::String("test".to_string())],
        };
        interpreter.execute_statement(&statement, &DummyBuiltins, &mut env)?;

        assert_eq!(env.output.len(), 1);
        assert_eq!(env.output[0], "WRITE: test");
        Ok(())
    }

    #[test]
    fn test_string_interpolation() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("freq".to_string(), Value::Integer(14500000));
        env.set(
            "vfo".to_string(),
            Value::EnumVariant {
                enum_name: "Vfo".to_string(),
                variant_name: "A".to_string(),
                value: 1,
            },
        );

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

        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(
            result,
            Value::Bytes(
                [
                    0xFE, 0xFE, 0x94, 0xE0, 0x25, 0x01, 0xA0, 0x40, 0xDD, 0x00, 0xFD
                ]
                .to_vec()
            )
        );
        Ok(())
    }

    #[test]
    fn test_if_statement() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("x".to_string(), Value::Integer(10));

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

        interpreter.execute_statement(&statement, &DummyBuiltins, &mut env)?;

        assert_eq!(env.output.len(), 1);
        assert_eq!(env.output[0], "WRITE: condition true");
        Ok(())
    }

    #[test]
    fn test_enum_handling() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let enum_def = crate::parser::Enum {
            name: "Vfo".to_string(),
            variants: BTreeMap::from([("A".to_string(), 0), ("B".to_string(), 1)]),
        };
        env.register_enum(&enum_def);

        let expr = Expr::QualifiedIdentifier(Id::new("Vfo"), Id::new("A"));
        let result = interpreter.evaluate_expression(&expr, &mut env)?;

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
                    command = "FEFE94E0.25.{vfo:1}.{freq:4}.FD";
                    write(command);
                }
            }
        "#;

        let rig_file = parse(dsl_source).unwrap();
        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = interpreter.create_env().unwrap();
        interpreter.execute_init(&DummyBuiltins, &mut env).unwrap();

        assert_eq!(env.get("version"), Some(Value::Integer(1)));
        assert_eq!(env.get("baudrate"), Some(Value::Integer(9600)));

        assert_eq!(env.get_enum_variant("Vfo", "A"), Some(0));
        assert_eq!(env.get_enum_variant("Vfo", "B"), Some(1));

        assert!(env.output.len() == 1);

        let args = vec![
            Value::Integer(14500000),
            Value::EnumVariant {
                enum_name: "Vfo".to_string(),
                variant_name: "A".to_string(),
                value: 0,
            },
        ];
        interpreter.execute_command("set_freq", &args, &DummyBuiltins, &mut env)?;

        assert_eq!(env.output[0], "WRITE: initialization");

        let last_output = env.output.last().unwrap();
        assert!(last_output.contains("WRITE: [254, 254, 148, 224, 37, 0, 160, 64, 221, 0, 253]"));
        Ok(())
    }

    #[test]
    fn test_division_by_zero_error() {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Divide,
            right: Box::new(Expr::Integer(0)),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.to_string().to_lowercase().contains("division")
                || error.to_string().to_lowercase().contains("zero")
        );
    }

    #[test]
    fn test_modulo_by_zero_error() {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Integer(10)),
            op: BinaryOp::Modulo,
            right: Box::new(Expr::Integer(0)),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env);
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_variable_access() {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::Identifier(Id::new("undefined_variable"));
        let result = interpreter.evaluate_expression(&expr, &mut env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("undefined") || error.to_string().contains("not found"));
    }

    #[test]
    fn test_complex_nested_expressions() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("a".to_string(), Value::Integer(10));
        env.set("b".to_string(), Value::Integer(5));
        env.set("c".to_string(), Value::Integer(3));
        env.set("d".to_string(), Value::Integer(2));

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier(Id::new("a"))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Identifier(Id::new("b"))),
                }),
                op: BinaryOp::Multiply,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier(Id::new("c"))),
                    op: BinaryOp::Subtract,
                    right: Box::new(Expr::Identifier(Id::new("d"))),
                }),
            }),
            op: BinaryOp::Add,
            right: Box::new(Expr::Integer(5)),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Integer(20));
        Ok(())
    }

    #[test]
    fn test_operator_precedence_validation() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("a".to_string(), Value::Integer(2));
        env.set("b".to_string(), Value::Integer(3));
        env.set("c".to_string(), Value::Integer(4));

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Identifier(Id::new("a"))),
            op: BinaryOp::Add,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier(Id::new("b"))),
                op: BinaryOp::Multiply,
                right: Box::new(Expr::Identifier(Id::new("c"))),
            }),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Integer(14));
        Ok(())
    }

    #[test]
    fn test_float_integer_mixed_operations() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Float(3.5)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Integer(2)),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        match result {
            Value::Float(f) => assert!((f - 5.5).abs() < 1e-6),
            _ => panic!("Expected float result"),
        }
        Ok(())
    }

    #[test]
    fn test_nested_if_statements() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let nested_if = Statement::If {
            condition: Expr::BinaryOp {
                left: Box::new(Expr::Integer(5)),
                op: BinaryOp::Greater,
                right: Box::new(Expr::Integer(3)),
            },
            then_body: vec![
                Statement::FunctionCall {
                    name: "write".to_string(),
                    args: vec![Expr::String("nested_true".to_string())],
                },
                Statement::If {
                    condition: Expr::BinaryOp {
                        left: Box::new(Expr::Integer(1)),
                        op: BinaryOp::Equal,
                        right: Box::new(Expr::Integer(1)),
                    },
                    then_body: vec![Statement::FunctionCall {
                        name: "write".to_string(),
                        args: vec![Expr::String("deeply_nested".to_string())],
                    }],
                    else_body: None,
                },
            ],
            else_body: Some(vec![Statement::FunctionCall {
                name: "write".to_string(),
                args: vec![Expr::String("nested_false".to_string())],
            }]),
        };

        interpreter.execute_statement(&nested_if, &DummyBuiltins, &mut env)?;

        assert_eq!(env.output.len(), 2);
        assert_eq!(env.output[0], "WRITE: nested_true");
        assert_eq!(env.output[1], "WRITE: deeply_nested");
        Ok(())
    }

    #[test]
    fn test_complex_boolean_expressions() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let expr = Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Integer(1)),
                    op: BinaryOp::Equal,
                    right: Box::new(Expr::Integer(1)),
                }),
                op: BinaryOp::And,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Integer(1)),
                    op: BinaryOp::Equal,
                    right: Box::new(Expr::Integer(2)),
                }),
            }),
            op: BinaryOp::Or,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Integer(3)),
                op: BinaryOp::Greater,
                right: Box::new(Expr::Integer(2)),
            }),
        };

        let result = interpreter.evaluate_expression(&expr, &mut env)?;
        assert_eq!(result, Value::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_if_with_non_boolean_condition_error() {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let if_stmt = Statement::If {
            condition: Expr::Integer(42),
            then_body: vec![Statement::FunctionCall {
                name: "write".to_string(),
                args: vec![Expr::String("should not execute".to_string())],
            }],
            else_body: None,
        };

        let result = interpreter.execute_statement(&if_stmt, &DummyBuiltins, &mut env);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("boolean") || error.to_string().contains("condition"));
    }

    #[test]
    fn test_variable_scoping_in_commands() -> Result<()> {
        let dsl_source = r#"
            version = 1;
            global_var = 42;
            impl Test for Rig {
                fn test_command(int param) {
                    local_var = param + 10;
                    write("test");
                }
            }
        "#;

        let rig_file = parse(dsl_source)?;
        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = interpreter.create_env()?;

        assert_eq!(env.get("version"), Some(Value::Integer(1)));
        assert_eq!(env.get("global_var"), Some(Value::Integer(42)));

        let args = vec![Value::Integer(5)];
        interpreter.execute_command("test_command", &args, &DummyBuiltins, &mut env)?;

        assert_eq!(env.get("local_var"), None);
        assert_eq!(env.get("global_var"), Some(Value::Integer(42)));
        Ok(())
    }

    #[test]
    fn test_parameter_passing_to_functions() -> Result<()> {
        let dsl_source = r#"
            impl Test for Rig {
                fn test_params(int a, bool b) {
                    if b {
                        result = a * 2;
                    } else {
                        result = a;
                    }
                    write("executed");
                }
            }
        "#;

        let rig_file = parse(dsl_source)?;
        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = Env::new();

        let args = vec![Value::Integer(10), Value::Boolean(true)];
        interpreter.execute_command("test_params", &args, &DummyBuiltins, &mut env)?;

        assert_eq!(env.output.len(), 1);
        assert_eq!(env.output[0], "WRITE: executed");
        Ok(())
    }

    #[test]
    fn test_all_data_formats_in_interpolation() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        let test_cases = vec![
            ("int_lu", 418),
            ("int_ls", 418),
            ("int_bu", 418),
            ("int_bs", 418),
            ("bcd_lu", 418),
            ("bcd_ls", 418),
            ("bcd_bu", 418),
            ("bcd_bs", 418),
            ("text", 418),
        ];

        for (format, value) in test_cases {
            env.set("test_var".to_string(), Value::Integer(value));

            let parts = vec![
                InterpolationPart::Literal(vec![0xFE, 0xFE]),
                InterpolationPart::Variable {
                    name: "test_var".to_string(),
                    format: Some(format.to_string()),
                    length: 4,
                },
                InterpolationPart::Literal(vec![0xFD]),
            ];

            let expr = Expr::StringInterpolation { parts };
            let result = interpreter.evaluate_expression(&expr, &mut env);

            match result {
                Ok(Value::Bytes(_)) => {}
                Ok(_) => {
                    panic!("Expected string result for format {}", format);
                }
                Err(e) if format == "invalid_format" => {
                    assert!(e.to_string().contains("format") || e.to_string().contains("invalid"));
                }
                Err(e) => {
                    panic!("Unexpected error for format {}: {}", format, e);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_string_interpolation_with_invalid_format() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("test_var".to_string(), Value::Integer(418));

        let parts = vec![InterpolationPart::Variable {
            name: "test_var".to_string(),
            format: Some("invalid_format".to_string()),
            length: 4,
        }];

        let expr = Expr::StringInterpolation { parts };
        let result = interpreter.evaluate_expression(&expr, &mut env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("format") || error.to_string().contains("invalid"));
        Ok(())
    }

    #[test]
    fn test_string_interpolation_zero_length() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("test_var".to_string(), Value::Integer(1));

        let parts = vec![InterpolationPart::Variable {
            name: "test_var".to_string(),
            format: Some("int_lu".to_string()),
            length: 0,
        }];

        let expr = Expr::StringInterpolation { parts };
        let result = interpreter.evaluate_expression(&expr, &mut env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("too long") || error.to_string().contains("0 bytes"));
        Ok(())
    }

    #[test]
    fn test_string_interpolation_large_numbers() -> Result<()> {
        let interpreter = Interpreter::default();
        let mut env = Env::new();

        env.set("large_num".to_string(), Value::Integer(0x12345678));

        let parts = vec![InterpolationPart::Variable {
            name: "large_num".to_string(),
            format: Some("int_lu".to_string()),
            length: 4,
        }];

        let expr = Expr::StringInterpolation { parts };
        let result = interpreter.evaluate_expression(&expr, &mut env)?;

        match result {
            Value::Bytes(bytes) => {
                assert!(!bytes.is_empty());
            }
            _ => panic!("Expected string result"),
        }
        Ok(())
    }

    #[test]
    fn test_qualified_identifier_enum_access() -> Result<()> {
        let dsl_source = r#"
            version = 1;
            impl Test for Rig {
                enum TestEnum {
                    A = 10,
                    B = 20,
                }
                fn test() {
                    x = TestEnum::A;
                    y = TestEnum::B;
                    write("test");
                }
            }
        "#;

        let rig_file = parse(dsl_source)?;
        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = interpreter.create_env()?;

        let result = interpreter.execute_command("test", &[], &DummyBuiltins, &mut env);
        assert!(
            result.is_ok(),
            "Command with qualified identifiers should execute successfully"
        );

        assert_eq!(env.output.len(), 1);
        assert_eq!(env.output[0], "WRITE: test");
        Ok(())
    }

    #[test]
    fn test_undefined_enum_access() -> Result<()> {
        let dsl_source = r#"
            impl Test for Rig {
                enum TestEnum {
                    A = 10,
                }
                fn test() {
                    x = TestEnum::NonExistent;
                }
            }
        "#;

        let rig_file = parse(dsl_source)?;
        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = interpreter.create_env()?;

        let result = interpreter.execute_command("test", &[], &DummyBuiltins, &mut env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("variant") || error.to_string().contains("NonExistent"));
        Ok(())
    }

    #[test]
    fn test_string_interpolation_invalid_format() -> Result<()> {
        use crate::interpreter::{Interpreter, Value};

        let dsl_source = r#"
            impl Test for Rig {
                fn test() {
                    command = "FEFE{var:invalid_format:2}FD";
                    write(command);
                }
            }
        "#;

        let rig_file = parse(dsl_source)?;

        let interpreter = Interpreter::new(rig_file.clone());
        let mut env = interpreter.create_env()?;

        env.set("var".to_string(), Value::Integer(42));

        let result = interpreter.execute_command("test", &[], &DummyBuiltins, &mut env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error:\n{error}");
        assert!(
            error.to_string().to_lowercase().contains("format")
                || error.to_string().to_lowercase().contains("invalid")
        );
        Ok(())
    }
}
