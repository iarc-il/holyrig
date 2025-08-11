use anyhow::Result;

use holyrig::parser::{Statement, Token, rig};
use logos::Logos;

fn parse(source: &str) -> Result<Vec<Statement>> {
    let tokens: Vec<_> = Token::lexer(source)
        .filter(|token| {
            println!("Token: {token:?}");
            !matches!(token, Ok(Token::Comment) | Ok(Token::NewLine))
        })
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!(""))?;
    println!("Comment: {tokens:#?}");
    Ok(rig::rig_file(&tokens)?)
}

fn main() -> Result<()> {
    // let rig_file = std::fs::read_to_string("test_rig_syntax.rig")?;
    // println!("Hello world: {:?}", parse(&rig_file)?);
    println!("Hello world: {:?}", parse("version = 3;\n//Hello!\n")?);
    Ok(())
}
