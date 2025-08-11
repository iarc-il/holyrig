use std::collections::BTreeMap;

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
    #[token("init")]
    Init,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(String);

#[derive(Debug)]
pub enum Expr {
    Number(u32),
}

#[derive(Debug)]
pub enum Statement {
    Assign(Id, Expr),
}

#[derive(Debug)]
pub struct Init {

}

#[derive(Debug)]
pub struct Impl {
    schema: String,
    name: String,
    init: Init,
}

#[derive(Debug)]
pub struct Settings {
    settings: BTreeMap<Id, Expr>,
}

#[derive(Debug)]
pub struct RigFile {
    settings: Settings,
    impl_block: Impl,
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
                            let Statement::Assign(id, expr) = statement;
                            (id, expr)
                        })
                        .collect()
                }
            }

        rule init() -> Init
            = [Token::Init] [Token::BraceOpen] [Token::BraceClose] {
                Init {  }
            }

        rule impl_block() -> Impl
            =
                [Token::Impl]
                [Token::Id(schema)]
                [Token::For]
                [Token::Id(name)]
                [Token::BraceOpen]
                init:init()
                [Token::BraceClose]
            {
                Impl {
                    schema: schema.to_string(),
                    name: name.to_string(),
                    init,
                }
            }
        rule impl_rig() -> RigFile
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

        rule expr() -> Expr
            = [Token::Number(number)] {?
                Ok(Expr::Number(number.parse().or(Err("Not a number"))?))
            }

    }
}
