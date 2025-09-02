use anyhow::Result;

use holyrig::parse_and_validate_with_schema;

fn main() -> Result<()> {
    println!("Reading rig file");
    let rig_content = std::fs::read_to_string("rigs/IC-7300.rig")?;
    println!("Reading schema");
    let schema_content = std::fs::read_to_string("schema/transceiver.schema")?;

    match parse_and_validate_with_schema(&rig_content, &schema_content) {
        Ok(rig_file) => {
            println!("Successfully parsed IC7300.rig");
            println!(" - Schema: {}", rig_file.impl_block.schema);
            println!(" - Name: {}", rig_file.impl_block.name);
        }
        Err(errors) => {
            for err in errors {
                println!("{err}");
            }
        }
    }

    Ok(())
}
