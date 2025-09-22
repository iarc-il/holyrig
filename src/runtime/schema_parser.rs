use std::collections::BTreeMap;

use anyhow::Result;
use logos::Logos;

use super::parser::{DataType, Id, Token};
use super::parser_errors::{
    ErrorLevel, ParseError, ParseErrorType, SourcePosition, calculate_position,
};

#[derive(Debug, Clone)]
pub struct SchemaParameter {
    pub param_type: DataType,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum SchemaMember {
    Enum(String, Vec<String>),
    Command(String, Vec<SchemaParameter>),
    Status(Vec<SchemaParameter>),
}

#[derive(Debug, Clone)]
pub struct SchemaBlock {
    pub name: String,
    pub enums: BTreeMap<String, Vec<String>>,
    pub commands: BTreeMap<String, Vec<SchemaParameter>>,
    pub status: BTreeMap<String, DataType>,
}

#[derive(Debug, Clone)]
pub struct SchemaFile {
    pub version: u32,
    pub name: String,
    pub enums: BTreeMap<String, Vec<String>>,
    pub commands: BTreeMap<String, Vec<SchemaParameter>>,
    pub status: BTreeMap<String, DataType>,
}

peg::parser! {
    pub grammar schema_parser<'source>() for [Token<'source>] {
        rule integer() -> u32
            = [Token::Integer(num)] {?
                num.parse::<u32>().or(Err("Invalid integer"))
            }

        rule identifier() -> Id
            = [Token::Id(id)] {
                Id::new(id)
            }

        rule data_type() -> DataType
            = [Token::Int] { DataType::Int }
            / [Token::Bool] { DataType::Bool }
            / id:identifier() { DataType::Enum(id.as_str().to_string()) }

        rule parameter() -> SchemaParameter
            = param_type:data_type() name:identifier() {
                SchemaParameter {
                    param_type,
                    name: name.as_str().to_string(),
                }
            }

        rule parameter_list() -> Vec<SchemaParameter>
            = [Token::ParenOpen] params:(parameter() ** [Token::Comma]) [Token::ParenClose] {
                params
            }

        rule command_declaration() -> (String, Vec<SchemaParameter>)
            = [Token::Fn] name:identifier() params:parameter_list()? [Token::Semicolon] {
                (name.as_str().to_string(), params.unwrap_or_default())
            }

        rule enum_variant() -> String
            = id:identifier() [Token::Comma]? {
                id.as_str().to_string()
            }

        rule enum_declaration() -> SchemaMember
            = [Token::Enum] name:identifier() [Token::BraceOpen]
              variants:enum_variant()*
              [Token::BraceClose] {
                SchemaMember::Enum(name.as_str().to_string(), variants)
            }

        rule status_field() -> SchemaParameter
            = param_type:data_type() name:identifier() [Token::Semicolon] {
                SchemaParameter {
                    param_type,
                    name: name.as_str().to_string(),
                }
            }

        rule status_declaration() -> Vec<SchemaParameter>
            = [Token::Status] [Token::BraceOpen]
              fields:status_field()*
              [Token::BraceClose] {
                fields
            }

        rule schema_member() -> SchemaMember
            = enum_decl:enum_declaration() { enum_decl }
            / command:command_declaration() { SchemaMember::Command(command.0, command.1) }
            / status_decl:status_declaration() { SchemaMember::Status(status_decl) }

        rule schema_block() -> SchemaBlock
            = [Token::Schema] name:identifier() [Token::BraceOpen]
              members:schema_member()* [Token::BraceClose] {?

                let mut enums = BTreeMap::new();
                let mut commands = BTreeMap::new();
                let mut status = BTreeMap::new();
                for member in members {
                    match member {
                        SchemaMember::Enum(name, variants) => {
                            enums.insert(name, variants);
                        },
                        SchemaMember::Command(name, params) => {
                            commands.insert(name, params);
                        },
                        SchemaMember::Status(schema_status) => {
                            status.extend(
                                schema_status
                                    .into_iter()
                                    .map(|param| (param.name, param.param_type))
                            )
                        },
                    }
                }

                Ok(SchemaBlock {
                    name: name.as_str().to_string(),
                    enums,
                    commands,
                    status,
                })
            }

        rule version_setting() -> u32
            = version_keyword:identifier() [Token::EqualAssign] version:integer() [Token::Semicolon] {?
                if version_keyword.as_str() != "version" {
                    Err("Expected 'version' keyword")
                } else {
                    Ok(version)
                }
            }

        pub rule schema_file() -> SchemaFile
            = version:version_setting() block:schema_block() {
                SchemaFile {
                    version,
                    name: block.name,
                    enums: block.enums,
                    commands: block.commands,
                    status: block.status
                }
            }
    }
}

pub fn parse_schema(source: &str) -> Result<SchemaFile, ParseError> {
    parse_schema_with_level(source, ErrorLevel::Normal)
}

pub fn parse_schema_with_level(source: &str, level: ErrorLevel) -> Result<SchemaFile, ParseError> {
    let mut lexer = Token::lexer(source);
    let mut tokens_with_positions = Vec::new();

    while let Some(token_result) = lexer.next() {
        match token_result {
            Ok(token) => {
                if !matches!(token, Token::Comment | Token::NewLine) {
                    let span = lexer.span();
                    let position = calculate_position(source, span.start);
                    tokens_with_positions
                        .push(super::parser::TokenWithPosition { token, position });
                }
            }
            Err(_) => {
                let span = lexer.span();
                let position = calculate_position(source, span.start);
                return Err(ParseError {
                    position,
                    error_type: Box::new(ParseErrorType::Tokenization {
                        message: format!("Unable to tokenize input at position {}", span.start),
                        context: "Invalid character or token".to_string(),
                    }),
                    source: source.to_string(),
                    level,
                });
            }
        }
    }

    let tokens: Vec<Token> = tokens_with_positions.iter().map(|t| t.token).collect();

    schema_parser::schema_file(&tokens).map_err(|peg_error| {
        let error_msg = format!("{peg_error}");

        let position = if peg_error.location < tokens_with_positions.len() {
            tokens_with_positions[peg_error.location].position.clone()
        } else if !tokens_with_positions.is_empty() {
            tokens_with_positions[0].position.clone()
        } else {
            SourcePosition::new(1, 1, 0)
        };

        let found = if peg_error.location < tokens_with_positions.len() {
            Some(format!(
                "{:?}",
                tokens_with_positions[peg_error.location].token
            ))
        } else {
            Some("unexpected token".to_string())
        };

        let expected = peg_error
            .expected
            .tokens()
            .map(|token| token.to_string())
            .collect();

        ParseError {
            position,
            error_type: Box::new(ParseErrorType::Syntax {
                expected,
                found,
                context: format!("Failed to parse schema file structure. PEG error: {error_msg}"),
                peg_error: Some(error_msg),
                user_friendly_message: None,
            }),
            source: source.to_string(),
            level,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_schema() -> Result<()> {
        let schema_source = r#"
        version = 1;

        schema Transceiver {
            enum Vfo {
                A,
                B,
                Unknown,
            }

            fn set_freq(int freq, Vfo target);
            fn clear_rit();

            status {
                int freq_a;
                bool transmit;
                Vfo vfo;
            }
        }
        "#;

        let schema = parse_schema(schema_source)?;

        assert_eq!(schema.version, 1);
        assert_eq!(schema.name, "Transceiver");

        assert!(schema.enums.contains_key("Vfo"));
        let vfo_enum = &schema.enums["Vfo"];
        assert_eq!(
            vfo_enum.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
            &["A", "B", "Unknown"]
        );

        assert!(schema.commands.contains_key("set_freq"));
        let set_freq_cmd = &schema.commands["set_freq"];
        assert_eq!(set_freq_cmd.len(), 2);
        assert_eq!(set_freq_cmd[0].name, "freq");
        assert!(matches!(set_freq_cmd[0].param_type, DataType::Int));

        assert!(schema.status.contains_key("freq_a"));
        assert!(matches!(schema.status["freq_a"], DataType::Int));

        Ok(())
    }

    #[test]
    fn test_parse_enum_only() {
        let schema_source = r#"
        version = 1;

        schema Test {
            enum Mode {
                USB,
                LSB,
            }
        }
        "#;

        let result = parse_schema(schema_source);
        assert!(result.is_ok());

        let schema = result.unwrap();
        assert!(schema.enums.contains_key("Mode"));
        assert_eq!(schema.enums["Mode"], vec!["USB", "LSB"]);
    }

    #[test]
    fn test_parse_command_without_params() {
        let schema_source = r#"
        version = 1;

        schema Test {
            fn simple_command();
        }
        "#;

        let result = parse_schema(schema_source);
        assert!(result.is_ok());

        let schema = result.unwrap();
        assert!(schema.commands.contains_key("simple_command"));
        assert!(schema.commands["simple_command"].is_empty());
    }
}
