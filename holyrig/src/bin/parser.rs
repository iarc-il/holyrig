use std::path::PathBuf;

use anyhow::Result;
use argh::FromArgs;

use holyrig::runtime::{parse_and_validate_with_schema, parse_rig_file, parse_schema};

#[derive(FromArgs)]
/// Command line tool for validating rig files and schema files
struct Args {
    #[argh(option)]
    /// rig file to validate
    rig: Option<PathBuf>,
    #[argh(option)]
    /// schema file to validate
    schema: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let rig = if let Some(rig) = args.rig {
        Some(std::fs::read_to_string(rig)?)
    } else {
        None
    };

    let schema = if let Some(schema) = args.schema {
        Some(std::fs::read_to_string(schema)?)
    } else {
        None
    };

    match (rig, schema) {
        (Some(rig), Some(schema)) => {
            let schema = parse_schema(&schema)?;
            match parse_and_validate_with_schema(&rig, &schema) {
                Ok(rig_file) => {
                    println!("Successfully parsed schema and rig!");
                    println!(" - Schema: {}", rig_file.impl_block.schema);
                    println!(" - Name: {}", rig_file.impl_block.name);
                }
                Err(errors) => {
                    for err in errors {
                        eprintln!("{err}");
                    }
                }
            }
        }
        (None, Some(schema)) => match parse_schema(&schema) {
            Ok(schema) => {
                println!("Successfully parsed schema \"{}\"!", schema.name);
            }
            Err(err) => {
                eprintln!("{err}");
            }
        },
        (Some(rig), None) => match parse_rig_file(&rig) {
            Ok(rig) => {
                println!("Successfully parsed rig!");
                println!(" - Schema: {}", rig.impl_block.schema);
                println!(" - Name: {}", rig.impl_block.name);
            }
            Err(err) => {
                eprintln!("{err}");
            }
        },
        (None, None) => {
            eprintln!("You must provide rig file, schema or both!");
        }
    }

    Ok(())
}
