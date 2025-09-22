use std::collections::HashMap;

use anyhow::Result;

use super::interpreter::{Interpreter, Value};

pub trait ExternalApi: Send + Sync {
    fn write(&self, data: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn read(&self, size: usize) -> impl Future<Output = Result<Vec<u8>>> + Send;
    fn set_var(&self, var: &str, value: Value) -> Result<()>;
}

pub trait RigWrapper: Send + Sync {
    fn execute_init(&self, external: &impl ExternalApi) -> impl Future<Output = Result<()>> + Send;
    fn execute_command(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> impl Future<Output = Result<HashMap<String, Value>>> + Send;
    fn execute_status(
        &self,
        external: &impl ExternalApi,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl RigWrapper for Interpreter {
    async fn execute_init(&self, external: &impl ExternalApi) -> Result<()> {
        let mut env = self.create_env()?;
        Interpreter::execute_init(self, external, &mut env).await
    }

    async fn execute_command(
        &self,
        name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> Result<HashMap<String, Value>> {
        let mut env = self.create_env()?;

        let args = self.eval_external_args(name, params, &mut self.create_env()?)?;
        Interpreter::execute_command(self, name, &args, external, &mut env).await?;

        Ok(HashMap::new())
    }

    async fn execute_status(&self, external: &impl ExternalApi) -> Result<()> {
        let mut env = self.create_env()?;
        Interpreter::execute_status(self, external, &mut env).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use parking_lot::RwLock;

    use super::*;
    use crate::runtime::parser;

    struct MockExternalApi {
        pub written_data: RwLock<Vec<Vec<u8>>>,
        pub read_responses: RwLock<Vec<Vec<u8>>>,
        pub set_vars: RwLock<BTreeMap<String, Value>>,
    }

    impl MockExternalApi {
        fn new() -> Self {
            Self {
                written_data: RwLock::new(Vec::new()),
                read_responses: RwLock::new(Vec::new()),
                set_vars: RwLock::new(BTreeMap::new()),
            }
        }

        fn add_read_response(&self, data: Vec<u8>) {
            self.read_responses.write().push(data);
        }
    }

    impl ExternalApi for MockExternalApi {
        async fn write(&self, data: &[u8]) -> Result<()> {
            self.written_data.write().push(data.to_vec());
            Ok(())
        }

        async fn read(&self, size: usize) -> Result<Vec<u8>> {
            let mut responses = self.read_responses.write();
            if responses.is_empty() {
                Ok(vec![0; size])
            } else {
                Ok(responses.remove(0))
            }
        }

        fn set_var(&self, var: &str, value: Value) -> Result<()> {
            self.set_vars.write().insert(var.to_string(), value);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_interpreter_wrapper_init() {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {
                    write("FEFE94E0FD");
                }
                fn test_command() {}
                status {}
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let result = RigWrapper::execute_init(&interpreter, &external).await;
        assert!(result.is_ok(), "Init should succeed: {:?}", result);

        // Check that write was called with the expected data
        let written_data = external.written_data.read();
        assert_eq!(written_data.len(), 1);
        assert_eq!(written_data[0], vec![0xFE, 0xFE, 0x94, 0xE0, 0xFD]);
    }

    #[tokio::test]
    async fn test_interpreter_wrapper_command() {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {}
                fn set_freq(int freq) {
                    write("FEFE94E025{freq:int_lu:4}FD");
                }
                status {}
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let mut params = HashMap::new();
        params.insert("freq".to_string(), "14500000".to_string());

        let result = RigWrapper::execute_command(&interpreter, "set_freq", params, &external).await;
        assert!(result.is_ok(), "Command should succeed: {:?}", result);

        // Check that write was called with the expected data
        let written_data = external.written_data.read();
        assert_eq!(written_data.len(), 1);
        // FEFE94E025 + freq(14500000 in int_lu:4) + FD
        let expected = vec![0xFE, 0xFE, 0x94, 0xE0, 0x25, 0xA0, 0x40, 0xDD, 0x00, 0xFD];
        assert_eq!(written_data[0], expected);
    }

    #[tokio::test]
    async fn test_interpreter_wrapper_status() {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {}
                fn test_command() {}
                status {
                    write("FEFE94E003FD");
                }
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let result = RigWrapper::execute_status(&interpreter, &external).await;
        assert!(result.is_ok(), "Status should succeed: {:?}", result);

        // Check that write was called
        let written_data = external.written_data.read();
        assert_eq!(written_data.len(), 1);
        assert_eq!(written_data[0], vec![0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD]);
    }

    #[tokio::test]
    async fn test_parse_function_with_template() -> Result<()> {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {}
                fn test_command() {}
                status {
                    write("FEFE94E025FD");
                    read("FEFE94E0.25.{freq:bcd_lu:4}.FD");
                    set_var(s"freq", freq);
                }
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        external.add_read_response(vec![
            0xFE, 0xFE, 0x94, 0xE0, 0x25, 0x12, 0x34, 0x56, 0x78, 0xFD,
        ]);

        RigWrapper::execute_status(&interpreter, &external).await?;

        let var = external.set_vars.read().get("freq").cloned();
        assert_eq!(var, Some(Value::Integer(78563412)));
        Ok(())
    }

    #[tokio::test]
    async fn test_interpreter_wrapper_with_read() -> Result<()> {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {
                    write("FEFE94E0FD");
                    read("FEFE94E0FBFD");
                }
                fn test_command() {}
                status {}
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        external.add_read_response(vec![0xFE, 0xFE, 0x94, 0xE0, 0xFB, 0xFD]);

        RigWrapper::execute_init(&interpreter, &external).await?;

        let written_data = external.written_data.read();
        assert_eq!(written_data.len(), 1);
        assert_eq!(written_data[0], vec![0xFE, 0xFE, 0x94, 0xE0, 0xFD]);
        Ok(())
    }

    #[tokio::test]
    async fn test_interpreter_wrapper_missing_command() {
        let dsl_source = r#"
            version = 1;

            impl TestSchema for TestRig {
                init {}
                fn existing_command() {}
                status {}
            }
        "#;

        let rig_file = parser::parse_rig_file(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let result = RigWrapper::execute_command(
            &interpreter,
            "nonexistent_command",
            HashMap::new(),
            &external,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown command"));
    }
}
