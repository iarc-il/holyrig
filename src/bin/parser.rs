use anyhow::Result;

use holyrig::parser;

fn main() -> Result<()> {
    let rig_file_content = std::fs::read_to_string("rigs/IC7300.rig")?;

    match parser::parse(&rig_file_content) {
        Ok(rig_file) => {
            println!("Successfully parsed RigFile: {rig_file:#?}");
        }
        Err(e) => {
            println!("Failed to parse DSL: {e}");
        }
    }

    Ok(())
}
