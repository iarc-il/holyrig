use std::collections::HashMap;

use anyhow::Result;

use crate::{commands::Value, parser::RigFile, rig_api::RigApi};

pub trait ExternalApi {
    fn write(&self, data: &[u8]) -> Result<()>;
    fn read(&self, size: usize) -> Result<Vec<u8>>;
    fn set_var(&self, var: &str, value: Value) -> Result<()>;
}

pub trait RigWrapper {
    fn execute_init(&self, external: &impl ExternalApi) -> Result<()>;
    fn execute_command(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> Result<HashMap<String, Value>>;
    fn execute_status(&self, external: &impl ExternalApi) -> Result<()>;
}

impl RigWrapper for RigApi {
    fn execute_init(&self, external: &impl ExternalApi) -> Result<()> {
        for (index, data) in self.build_init_commands()?.into_iter().enumerate() {
            let expected_length = self.get_init_response_length(index)?.unwrap();

            external.write(&data)?;
            let response = external.read(expected_length)?;
            self.validate_init_response(index, &response)?;
        }
        Ok(())
    }

    fn execute_command(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> Result<HashMap<String, Value>> {
        let params = self.parse_param_values(command_name, params)?;
        let data = self.build_command(command_name, &params)?;
        let expected_length = self.get_command_response_length(command_name)?.unwrap();

        external.write(&data)?;
        let response = external.read(expected_length)?;
        self.parse_command_response(command_name, &response)
            .map_err(|err| anyhow::anyhow!(err))
    }

    fn execute_status(&self, external: &impl ExternalApi) -> Result<()> {
        let status_commands = self.get_status_commands()?;

        for (index, data) in status_commands.into_iter().enumerate() {
            let expected_length = self.get_status_response_length(index)?.unwrap();

            external.write(&data)?;
            let response = external.read(expected_length)?;

            let values = self
                .parse_status_response(index, &response)
                .map_err(|err| anyhow::anyhow!(err))?;

            // Store each value via set_var
            for (name, value) in values {
                external.set_var(&name, value)?;
            }
        }
        Ok(())
    }
}

impl RigWrapper for RigFile {
    fn execute_init(&self, _external: &impl ExternalApi) -> Result<()> {
        if let Some(_init) = &self.impl_block.init {
            // TODO
        }
        Ok(())
    }

    fn execute_command(
        &self,
        command_name: &str,
        _params: HashMap<String, String>,
        _external: &impl ExternalApi,
    ) -> Result<HashMap<String, Value>> {
        if !self.impl_block.commands.is_empty() {
            // TODO
        }

        Err(anyhow::anyhow!(
            "Command '{}' not found in DSL RigFile (DSL parsing not yet complete)",
            command_name
        ))
    }

    fn execute_status(&self, _external: &impl ExternalApi) -> Result<()> {
        if let Some(_status) = &self.impl_block.status {
            // TODO
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    struct MockExternalApi;

    impl ExternalApi for MockExternalApi {
        fn write(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }

        fn read(&self, _size: usize) -> Result<Vec<u8>> {
            Ok(vec![])
        }

        fn set_var(&self, _var: &str, _value: Value) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_dsl_rig_file_wrapper() {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {}
                fn {}
                status {}
            }
        "#;

        let rig_file = parser::parse(dsl_source).expect("Failed to parse DSL");
        let external = MockExternalApi;

        assert!(rig_file.execute_init(&external).is_ok());
        assert!(rig_file.execute_status(&external).is_ok());
        let result = rig_file.execute_command("test_command", HashMap::new(), &external);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in DSL RigFile")
        );
        assert_eq!(rig_file.impl_block.schema, "TestSchema");
        assert_eq!(rig_file.impl_block.name, "TestRig");
        assert!(rig_file.impl_block.init.is_some());
        assert!(rig_file.impl_block.status.is_some());
        assert_eq!(rig_file.impl_block.commands.len(), 1);
    }

    #[test]
    fn test_dsl_rig_file_wrapper_complex() {
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

        let rig_file = parser::parse(dsl_source).expect("Failed to parse complex DSL");
        let external = MockExternalApi;

        assert!(rig_file.execute_init(&external).is_ok());
        assert!(rig_file.execute_status(&external).is_ok());

        let result = rig_file.execute_command("unknown", HashMap::new(), &external);
        assert!(result.is_err());

        assert_eq!(rig_file.impl_block.schema, "Transceiver");
        assert_eq!(rig_file.impl_block.name, "IC7300");
        assert!(rig_file.impl_block.init.is_some());
        assert!(rig_file.impl_block.status.is_some());
        assert_eq!(rig_file.impl_block.commands.len(), 2);
        assert_eq!(rig_file.impl_block.enums.len(), 1);
        assert_eq!(rig_file.settings.settings.len(), 2);
    }
}
