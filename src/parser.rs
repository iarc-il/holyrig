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
                fn {}
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
        assert_eq!(rig_file.impl_block.enums.len(), 0);
        assert_eq!(rig_file.settings.settings.len(), 1);
    }

    #[test]
    fn test_parse_complex_dsl() {
        let dsl_source = r#"
            version = 2;
            baudrate = 9600;
            impl Transceiver for IC7300 {
                enum {}
                init {}
                fn {}
                fn {}
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
        assert_eq!(rig_file.settings.settings.len(), 2);
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
                fn {}
                fn {}
                fn {}
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
}
