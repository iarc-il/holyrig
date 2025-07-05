use anyhow::{Context, Result};
use argh::FromArgs;
use holyrig::{omnirig_parser::parse_ini_file, translator::translate_omnirig_to_rig};
use std::{fs, path::PathBuf};

/// Convert OmniRig .ini files to HolyRig .toml format
#[derive(FromArgs)]
struct Args {
    /// input OmniRig .ini file
    #[argh(option, short = 'i')]
    input: PathBuf,

    /// output HolyRig .toml file
    #[argh(option, short = 'o')]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let rig_desc = parse_ini_file(&args.input)
        .with_context(|| format!("Failed to parse input file: {}", args.input.display()))?;
    let rig_data = translate_omnirig_to_rig(rig_desc);
    let toml_string =
        toml::to_string_pretty(&rig_data).with_context(|| "Failed to serialize to TOML")?;
    fs::write(&args.output, toml_string)
        .with_context(|| format!("Failed to write output file: {}", args.output.display()))?;

    println!(
        "Successfully converted {} to {}",
        args.input.display(),
        args.output.display()
    );
    Ok(())
}
