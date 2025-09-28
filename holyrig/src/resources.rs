use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

use crate::runtime::{Interpreter, SchemaFile, parse_and_validate_with_schema, parse_schema};

pub struct Resources {
    pub schema: SchemaFile,
    pub rigs: HashMap<String, Interpreter>,
}

impl Resources {
    pub fn load() -> Result<Arc<Self>> {
        let schema = Self::load_schema()?;
        let rigs = Self::load_rig_files(&schema)?;
        Ok(Arc::new(Self { schema, rigs }))
    }

    fn load_schema() -> Result<SchemaFile> {
        let schema_path = if cfg!(debug_assertions) {
            std::path::PathBuf::from("../schema/transceiver.schema")
        } else {
            dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
                .join("holyrig")
                .join("schema.toml")
        };
        Ok(parse_schema(&std::fs::read_to_string(schema_path)?)?)
    }

    fn load_rig_files(schema: &SchemaFile) -> Result<HashMap<String, Interpreter>> {
        let mut rigs = HashMap::new();

        for entry in std::fs::read_dir("../rigs")? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rig") {
                continue;
            }

            let file_name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
                .to_string();

            let content = std::fs::read_to_string(path)?;
            let rig_file = parse_and_validate_with_schema(&content, schema).unwrap();
            rigs.insert(file_name, Interpreter::new(rig_file));
        }

        Ok(rigs)
    }
}
