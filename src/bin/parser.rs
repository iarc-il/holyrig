use anyhow::Result;

use holyrig::parser;

fn main() -> Result<()> {
    let rig_file_content = std::fs::read_to_string("rigs/IC7300.rig")?;

    match parser::parse(&rig_file_content) {
        Ok(rig_file) => {
            println!("Successfully parsed IC7300.rig");
            println!(" - Schema: {}", rig_file.impl_block.schema);
            println!(" - Name: {}", rig_file.impl_block.name);
        }
        Err(err) => {
            println!("{err}");
        }
    }

    Ok(())
}
