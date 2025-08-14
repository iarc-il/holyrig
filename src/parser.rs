use std::collections::BTreeMap;

use anyhow::Result;
use logos::Logos;

#[derive(Logos, Debug, Copy, Clone)]
#[logos(skip r"[ \t\f]+")]
pub enum Token<'source> {
    #[regex(r"[a-zA-Z_][a-zA-Z_0-9]*", |lex| lex.slice())]
    Id(&'source str),
    #[regex(r"[0-9]+", |lex| lex.slice())]
    Number(&'source str),
    #[regex("\"[^\"]*\"", |lex| lex.slice())]
    Str(&'source str),
    #[token("impl")]
    Impl,
    #[token("for")]
    For,
    #[token("enum")]
    Enum,
    #[token("init")]
    Init,
    #[token("fn")]
    Fn,
    #[token("status")]
    Status,
    #[token("int")]
    Int,
    #[token("bool")]
    Bool,
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("=")]
    Equal,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("::")]
    DoubleColon,
    #[token("\n")]
    NewLine,
    #[regex(r"//[^\n]*\n")]
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(String);

impl From<String> for Id {
    fn from(value: String) -> Self {
        Id(value)
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(u32),
    String(String),
    Identifier(Id),
    QualifiedIdentifier(Id, Id),
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    StringInterpolation {
        template: String,
        variables: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assign(Id, Expr),
    FunctionCall { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct Init {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,
    Bool,
    Enum(String),
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub param_type: DataType,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Member {
    Enum(Enum),
    Init(Init),
    Command(Command),
    Status(Status),
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub schema: String,
    pub name: String,
    pub init: Option<Init>,
    pub status: Option<Status>,
    pub commands: Vec<Command>,
    pub enums: Vec<Enum>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub settings: BTreeMap<Id, Expr>,
}

#[derive(Debug, Clone)]
pub struct RigFile {
    pub settings: Settings,
    pub impl_block: Impl,
}

peg::parser! {
    pub grammar rig<'source>() for [Token<'source>] {
        pub rule rig_file() -> Vec<Statement>
            = assigns:assign()+ { assigns }

        rule settings() -> Settings
            = assigns:assign()* {
                Settings {
                    settings: assigns
                        .into_iter()
                        .map(|statement| {
                            match statement {
                                Statement::Assign(id, expr) => (id, expr),
                                _ => panic!("Expected assign statement in settings"),
                            }
                        })
                        .collect()
                }
            }

        rule enum_variant() -> EnumVariant
            = [Token::Id(name)] [Token::Equal] [Token::Number(value)] {?
                Ok(EnumVariant {
                    name: name.to_string(),
                    value: value.parse().or(Err("Not a number"))?,
                })
            }

        rule enum_member() -> Member
            = [Token::Enum] [Token::Id(name)] [Token::BraceOpen]
              variants:(enum_variant() ** [Token::Comma]) [Token::Comma]?
              [Token::BraceClose] {
                Member::Enum(Enum {
                    name: name.to_string(),
                    variants,
                })
            }

        rule parameter() -> Parameter
            = param_type:(
                [Token::Int] { DataType::Int } /
                [Token::Bool] { DataType::Bool } /
                [Token::Id(data_type)] { DataType::Enum(data_type.to_string()) }
            ) [Token::Id(name)] {
                Parameter {
                    param_type,
                    name: name.to_string(),
                }
            }

        rule statement() -> Statement
            = function_call_stmt() / var_assign_statement()

        rule function_call_stmt() -> Statement
            = [Token::Id(name)] [Token::ParenOpen]
              args:(expr() ** [Token::Comma]) [Token::Comma]?
              [Token::ParenClose] [Token::Semicolon] {
                Statement::FunctionCall {
                    name: name.to_string(),
                    args
                }
            }

        rule var_assign_statement() -> Statement
            = [Token::Id(var)] [Token::Equal] expr:expr() [Token::Semicolon] {
                Statement::Assign(Id(var.to_string()), expr)
            }

        rule init() -> Member
            = [Token::Init] [Token::BraceOpen] statements:statement()* [Token::BraceClose] {
                Member::Init(Init { statements })
            }

        rule command() -> Member
            = [Token::Fn] [Token::Id(name)] [Token::ParenOpen]
              params:(parameter() ** [Token::Comma]) [Token::Comma]?
              [Token::ParenClose] [Token::BraceOpen]
              statements:statement()*
              [Token::BraceClose] {
                Member::Command(Command {
                    name: name.to_string(),
                    parameters: params,
                    statements,
                })
            }

        rule status() -> Member
            = [Token::Status] [Token::BraceOpen] statements:statement()* [Token::BraceClose] {
                Member::Status(Status { statements })
            }

        rule member() -> Member
            = member:(init() / enum_member() / command() / status()) {
            member
        }

        rule impl_block() -> Impl
            =
                [Token::Impl]
                [Token::Id(schema)]
                [Token::For]
                [Token::Id(name)]
                [Token::BraceOpen]
                members:member()*
                [Token::BraceClose]
            {
                let mut init = None;
                let mut status = None;
                let mut commands = Vec::new();
                let mut enums = Vec::new();

                for member in members {
                    match member {
                        Member::Init(i) => init = Some(i),
                        Member::Status(s) => status = Some(s),
                        Member::Command(c) => commands.push(c),
                        Member::Enum(e) => enums.push(e),
                    }
                }

                Impl {
                    schema: schema.to_string(),
                    name: name.to_string(),
                    init,
                    status,
                    commands,
                    enums,
                }
            }
        pub rule impl_rig() -> RigFile
            = settings:settings() impl_block:impl_block() {
                RigFile {
                    settings,
                    impl_block,
                }
            }

        rule assign() -> Statement
            = [Token::Id(id)] [Token::Equal] expr:expr() [Token::Semicolon] {
                Statement::Assign(Id(id.into()), expr)
            }

        rule method_call() -> Expr
            = object:primary_expr() [Token::Dot] [Token::Id(method)] [Token::ParenOpen]
              args:(expr() ** [Token::Comma]) [Token::Comma]?
              [Token::ParenClose] {
                Expr::MethodCall {
                    object: Box::new(object),
                    method: method.to_string(),
                    args,
                }
            }

        rule primary_expr() -> Expr
            = [Token::Number(number)] {?
                Ok(Expr::Number(number.parse().or(Err("Not a number"))?))
            }
            / [Token::Str(s)] {
                // Handle string interpolation
                let content = &s[1..s.len()-1]; // Remove quotes
                if content.contains('{') && content.contains('}') {
                    // Extract variables from {var} patterns
                    let mut variables = Vec::new();
                    let mut chars = content.chars();
                    let mut current_var = String::new();
                    let mut in_brace = false;

                    for ch in chars {
                        if ch == '{' {
                            in_brace = true;
                            current_var.clear();
                        } else if ch == '}' && in_brace {
                            if !current_var.is_empty() {
                                variables.push(current_var.clone());
                            }
                            in_brace = false;
                        } else if in_brace {
                            current_var.push(ch);
                        }
                    }

                    Expr::StringInterpolation {
                        template: content.to_string(),
                        variables,
                    }
                } else {
                    Expr::String(content.to_string())
                }
            }
            / [Token::Id(scope)] [Token::DoubleColon] [Token::Id(id)] {
                Expr::QualifiedIdentifier(scope.into(), id.into())
            }
            / [Token::Id(id)] {
                Expr::Identifier(id.into())
            }

        rule expr() -> Expr
            = method_call() / primary_expr()

    }
}

pub fn parse(source: &str) -> Result<RigFile> {
    let tokens: Vec<_> = Token::lexer(source)
        .filter(|token| !matches!(token, Ok(Token::Comment) | Ok(Token::NewLine)))
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("Failed to tokenize DSL string"))?;

    rig::impl_rig(&tokens).map_err(|e| anyhow::anyhow!("Failed to parse DSL: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dsl() {
        let dsl_source = r#"
            version = 1;
            impl TestSchema for TestRig {
                init {}
                fn test_command() {}
                status {}
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "TestSchema");
        assert_eq!(rig_file.impl_block.name, "TestRig");
        assert!(rig_file.impl_block.init.is_some());
        assert!(rig_file.impl_block.status.is_some());
        assert_eq!(rig_file.impl_block.commands.len(), 1);
        assert_eq!(rig_file.impl_block.commands[0].name, "test_command");
        assert_eq!(rig_file.impl_block.enums.len(), 0);
        assert_eq!(rig_file.settings.settings.len(), 1);
    }

    #[test]
    fn test_parse_complex_dsl() {
        let dsl_source = r#"
            version = 2;
            baudrate = 9600;
            impl Transceiver for IC7300 {
                enum TestEnum {
                    A = 0,
                    B = 1,
                }
                init {}
                fn command1() {}
                fn command2() {}
                status {}
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "Transceiver");
        assert_eq!(rig_file.impl_block.name, "IC7300");
        assert!(rig_file.impl_block.init.is_some());
        assert!(rig_file.impl_block.status.is_some());
        assert_eq!(rig_file.impl_block.commands.len(), 2);
        assert_eq!(rig_file.impl_block.enums.len(), 1);
        assert_eq!(rig_file.impl_block.enums[0].name, "TestEnum");
        assert_eq!(rig_file.impl_block.enums[0].variants.len(), 2);
        assert_eq!(rig_file.settings.settings.len(), 2);
    }

    #[test]
    fn test_parse_function_with_parameters() {
        let dsl_source = r#"
            impl Test for Rig {
                fn set_freq(int freq, bool enabled) {
                    write("test");
                    read("response");
                    command = "test_command";
                }
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.commands.len(), 1);
        let cmd = &rig_file.impl_block.commands[0];
        assert_eq!(cmd.name, "set_freq");
        assert_eq!(cmd.parameters.len(), 2);
        assert_eq!(cmd.parameters[0].param_type, DataType::Int);
        assert_eq!(cmd.parameters[0].name, "freq");
        assert_eq!(cmd.parameters[1].param_type, DataType::Bool);
        assert_eq!(cmd.parameters[1].name, "enabled");
        assert_eq!(cmd.statements.len(), 3);
    }

    #[test]
    fn test_parse_ic7300_subset() {
        let dsl_source = r#"
            version = 1;
            impl Transceiver for IC7300 {
                enum Vfo {
                    A = 0,
                    B = 1,
                }
                enum Mode {
                    LSB = 0,
                    USB = 1,
                    AM = 2,
                }
                init {
                    write("FEFE94E0.1A050071.00.FD");
                    read("FEFE94E01A05007100FD.FEFEE094FBFD");
                }
                fn set_freq(int freq, Vfo target) {
                    command = "FEFE94E0.25.{target}.{freq}.FD";
                    write(command);
                }
                status {}
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "Transceiver");
        assert_eq!(rig_file.impl_block.name, "IC7300");
        assert_eq!(rig_file.impl_block.enums.len(), 2);
        assert_eq!(rig_file.impl_block.commands.len(), 1);

        let vfo_enum = &rig_file.impl_block.enums[0];
        assert_eq!(vfo_enum.name, "Vfo");
        assert_eq!(vfo_enum.variants.len(), 2);
        assert_eq!(vfo_enum.variants[0].name, "A");
        assert_eq!(vfo_enum.variants[0].value, 0);

        let cmd = &rig_file.impl_block.commands[0];
        assert_eq!(cmd.name, "set_freq");
        assert_eq!(cmd.parameters.len(), 2);
        assert_eq!(
            cmd.parameters[1].param_type,
            DataType::Enum("Vfo".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_dsl() {
        let invalid_dsl = "invalid syntax here";
        let result = parse(invalid_dsl);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse DSL")
        );
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_minimal_structure() {
        let dsl_source = r#"
            impl Test for Minimal {
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "Test");
        assert_eq!(rig_file.impl_block.name, "Minimal");
        assert!(rig_file.impl_block.init.is_none());
        assert!(rig_file.impl_block.status.is_none());
        assert_eq!(rig_file.impl_block.commands.len(), 0);
        assert_eq!(rig_file.impl_block.enums.len(), 0);
        assert_eq!(rig_file.settings.settings.len(), 0);
    }

    #[test]
    fn test_parse_only_commands() {
        let dsl_source = r#"
            impl Test for Commands {
                fn cmd1() {}
                fn cmd2() {}
                fn cmd3() {}
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "Test");
        assert_eq!(rig_file.impl_block.name, "Commands");
        assert!(rig_file.impl_block.init.is_none());
        assert!(rig_file.impl_block.status.is_none());
        assert_eq!(rig_file.impl_block.commands.len(), 3);
        assert_eq!(rig_file.impl_block.enums.len(), 0);
    }

    #[test]
    fn test_parse_real_ic7300_file() {
        let ic7300_content =
            std::fs::read_to_string("rigs/IC7300.rig").expect("Failed to read IC7300.rig");

        let result = parse(&ic7300_content);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.schema, "Transceiver");
        assert_eq!(rig_file.impl_block.name, "IC7300");
        assert!(rig_file.impl_block.init.is_some());
        assert!(rig_file.impl_block.status.is_some());
        assert_eq!(rig_file.impl_block.enums.len(), 2);

        // Check that we have the expected functions
        let function_names: Vec<&str> = rig_file
            .impl_block
            .commands
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(function_names.contains(&"set_freq"));
        assert!(function_names.contains(&"cw_pitch"));
        assert!(function_names.contains(&"set_mode"));
    }

    #[test]
    fn test_identifiers_not_tokens() {
        let dsl_source = r#"
            impl Test for Rig {
                fn test_func() {
                    write("test");
                    read("response");
                    command = "test_command";
                    freq = freq.format(fmt::BcdLu, 5);
                }
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        assert_eq!(rig_file.impl_block.commands.len(), 1);
        let cmd = &rig_file.impl_block.commands[0];
        assert_eq!(cmd.name, "test_func");
        assert_eq!(cmd.statements.len(), 4);

        // Verify that write, read, command, format, and fmt are parsed as identifiers/function calls
        match &cmd.statements[0] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "write");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::String(s) => assert_eq!(s, "test"),
                    _ => panic!("Expected string for write"),
                }
            }
            _ => panic!("Expected function call for write"),
        }

        match &cmd.statements[1] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "read");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::String(s) => assert_eq!(s, "response"),
                    _ => panic!("Expected string for read"),
                }
            }
            _ => panic!("Expected function call for read"),
        }

        match &cmd.statements[2] {
            Statement::Assign(var, _) => {
                assert_eq!(var.0, "command"); // command parsed as identifier
            }
            _ => panic!("Expected variable assignment"),
        }

        match &cmd.statements[3] {
            Statement::Assign(var, expr) => {
                assert_eq!(var.0, "freq");
                // Verify the method call contains format and fmt as identifiers
                match expr {
                    Expr::MethodCall { method, args, .. } => {
                        assert_eq!(method, "format");
                        assert_eq!(args.len(), 2);
                        assert_eq!(
                            args[0],
                            Expr::QualifiedIdentifier(Id::from("fmt"), Id::from("BcdLu"))
                        );
                    }
                    _ => panic!("Expected method call"),
                }
            }
            _ => panic!("Expected variable assignment"),
        }
    }

    #[test]
    fn test_generic_function_calls() {
        let dsl_source = r#"
            impl Test for Rig {
                fn test_func() {
                    write("data");
                    read("response");
                    send_command("AT", "OK");
                    delay(100);
                    custom_func();
                }
            }
        "#;

        let result = parse(dsl_source);
        assert!(result.is_ok());

        let rig_file = result.unwrap();
        let cmd = &rig_file.impl_block.commands[0];
        assert_eq!(cmd.statements.len(), 5);

        // Test write with 1 argument
        match &cmd.statements[0] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "write");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected function call"),
        }

        // Test read with 1 argument
        match &cmd.statements[1] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "read");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected function call"),
        }

        // Test custom function with 2 arguments
        match &cmd.statements[2] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "send_command");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected function call"),
        }

        // Test function with numeric argument
        match &cmd.statements[3] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "delay");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::Number(n) => assert_eq!(*n, 100),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected function call"),
        }

        // Test function with no arguments
        match &cmd.statements[4] {
            Statement::FunctionCall { name, args } => {
                assert_eq!(name, "custom_func");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected function call"),
        }
    }
}
