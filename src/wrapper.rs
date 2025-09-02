use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    Env,
    commands::Value,
    data_format::DataFormat,
    interpreter::{Builtins, Interpreter, Value as InterpreterValue},
    parser::{Expr, InterpolationPart},
    rig_api::RigApi,
};

pub trait ExternalApi: Send + Sync {
    fn write(&self, data: &[u8]) -> impl std::future::Future<Output = Result<()>> + Send;
    fn read(&self, size: usize) -> impl std::future::Future<Output = Result<Vec<u8>>> + Send;
    fn set_var(&self, var: &str, value: Value) -> Result<()>;
}

pub trait RigWrapper: Send + Sync {
    fn execute_init(
        &self,
        external: &impl ExternalApi,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn execute_command(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> impl std::future::Future<Output = Result<HashMap<String, Value>>> + Send;
    fn execute_status(
        &self,
        external: &impl ExternalApi,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl RigWrapper for RigApi {
    async fn execute_init(&self, external: &impl ExternalApi) -> Result<()> {
        for (index, data) in self.build_init_commands()?.into_iter().enumerate() {
            let expected_length = self.get_init_response_length(index)?.unwrap();

            external.write(&data).await?;
            let response = external.read(expected_length).await?;
            self.validate_init_response(index, &response)?;
        }
        Ok(())
    }

    async fn execute_command(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
        external: &impl ExternalApi,
    ) -> Result<HashMap<String, Value>> {
        let params = self.parse_param_values(command_name, params)?;
        let data = self.build_command(command_name, &params)?;
        let expected_length = self.get_command_response_length(command_name)?.unwrap();

        external.write(&data).await?;
        let response = external.read(expected_length).await?;
        self.parse_command_response(command_name, &response)
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn execute_status(&self, external: &impl ExternalApi) -> Result<()> {
        let status_commands = self.get_status_commands()?;

        for (index, data) in status_commands.into_iter().enumerate() {
            let expected_length = self.get_status_response_length(index)?.unwrap();

            external.write(&data).await?;
            let response = external.read(expected_length).await?;

            let values = self
                .parse_status_response(index, &response)
                .map_err(|err| anyhow::anyhow!(err))?;

            for (name, value) in values {
                external.set_var(&name, value)?;
            }
        }
        Ok(())
    }
}

impl<E: ExternalApi> Builtins for E {
    async fn call(
        &self,
        name: &str,
        args: &[InterpreterValue],
        _env: &mut Env,
    ) -> Result<InterpreterValue> {
        match name {
            "write" => {
                let [InterpreterValue::Bytes(bytes)] = args else {
                    bail!("Expected one bytes argument in write, got: {args:?}");
                };
                self.write(bytes).await?;
                Ok(InterpreterValue::Unit)
            }
            "set_var" => {
                let [InterpreterValue::String(var), value] = args else {
                    bail!("Expected one bytes argument in write, got: {args:?}");
                };

                let value = match value {
                    InterpreterValue::Integer(int) => Value::Int(*int),
                    InterpreterValue::Boolean(bool) => Value::Bool(*bool),
                    InterpreterValue::EnumVariant {
                        enum_name: _,
                        variant_name,
                        value: _,
                    } => Value::Enum(variant_name.clone()),
                    other => todo!("{:?}", other),
                };

                self.set_var(var, value)?;
                Ok(InterpreterValue::Unit)
            }
            "read" => {
                bail!("Function read is not supported in this context, use call_no_eval");
            }
            _ => Err(anyhow!("Unknown function: {name}")),
        }
    }

    async fn call_no_eval(
        &self,
        name: &str,
        args: &[Expr],
        env: &mut Env,
    ) -> Result<InterpreterValue> {
        if name == "read" {
            match args {
                [Expr::StringInterpolation { parts }] => {
                    let expected_length = calculate_template_length(parts);
                    let response = self.read(expected_length).await?;
                    println!("Read: {response:?}");

                    parse_response_with_template(parts, &response, env)?;
                }
                [Expr::Bytes(bytes)] => {
                    let response = self.read(bytes.len()).await?;
                    if &response != bytes {
                        bail!("Got invalid response: {response:?}");
                    }
                }
                _ => {
                    bail!("Expected template string in parse, got: {args:?}");
                }
            };
            Ok(InterpreterValue::Unit)
        } else {
            bail!("Unknown function {name} in this context");
        }
    }
}

fn calculate_template_length(parts: &[InterpolationPart]) -> usize {
    parts
        .iter()
        .map(|part| match part {
            InterpolationPart::Literal(bytes) => bytes.len(),
            InterpolationPart::Variable { length, .. } => *length,
        })
        .sum()
}

fn parse_response_with_template(
    parts: &[InterpolationPart],
    response: &[u8],
    env: &mut Env,
) -> Result<()> {
    let mut offset = 0;

    for part in parts {
        match part {
            InterpolationPart::Literal(expected_bytes) => {
                if offset + expected_bytes.len() > response.len() {
                    bail!(
                        "Response too short: expected {} bytes at offset {}",
                        expected_bytes.len(),
                        offset
                    );
                }

                let actual = &response[offset..offset + expected_bytes.len()];
                if actual != expected_bytes {
                    bail!(
                        "Response doesn't match template at offset {}: expected {:?}, got {:?}",
                        offset,
                        expected_bytes,
                        actual
                    );
                }
                offset += expected_bytes.len();
            }
            InterpolationPart::Variable {
                name,
                format,
                length,
            } => {
                if offset + length > response.len() {
                    bail!(
                        "Response too short: expected {} bytes at offset {}",
                        length,
                        offset
                    );
                }

                let bytes = &response[offset..offset + length];
                let format_str = format.as_deref().unwrap_or("int_lu");
                let data_format = DataFormat::try_from(format_str)
                    .context(format!("Invalid format: {}", format_str))?;
                let value = data_format.decode(bytes).context(format!(
                    "Failed to decode {} bytes using format {}",
                    length, format_str
                ))?;

                if name != "_" {
                    env.set(name.clone(), InterpreterValue::Integer(value as i64));
                }
                offset += length;
            }
        }
    }

    Ok(())
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
    use std::{collections::BTreeMap, sync::RwLock};

    use super::*;
    use crate::parser;

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
            self.read_responses.write().unwrap().push(data);
        }
    }

    impl ExternalApi for MockExternalApi {
        async fn write(&self, data: &[u8]) -> Result<()> {
            self.written_data.write().unwrap().push(data.to_vec());
            Ok(())
        }

        async fn read(&self, size: usize) -> Result<Vec<u8>> {
            let mut responses = self.read_responses.write().unwrap();
            if responses.is_empty() {
                Ok(vec![0; size])
            } else {
                Ok(responses.remove(0))
            }
        }

        fn set_var(&self, var: &str, value: Value) -> Result<()> {
            self.set_vars
                .write()
                .unwrap()
                .insert(var.to_string(), value);
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

        let rig_file = parser::parse(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let result = RigWrapper::execute_init(&interpreter, &external).await;
        assert!(result.is_ok(), "Init should succeed: {:?}", result);

        // Check that write was called with the expected data
        let written_data = external.written_data.read().unwrap();
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

        let rig_file = parser::parse(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let mut params = HashMap::new();
        params.insert("freq".to_string(), "14500000".to_string());

        let result = RigWrapper::execute_command(&interpreter, "set_freq", params, &external).await;
        assert!(result.is_ok(), "Command should succeed: {:?}", result);

        // Check that write was called with the expected data
        let written_data = external.written_data.read().unwrap();
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

        let rig_file = parser::parse(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        let result = RigWrapper::execute_status(&interpreter, &external).await;
        assert!(result.is_ok(), "Status should succeed: {:?}", result);

        // Check that write was called
        let written_data = external.written_data.read().unwrap();
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

        let rig_file = parser::parse(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        external.add_read_response(vec![
            0xFE, 0xFE, 0x94, 0xE0, 0x25, 0x12, 0x34, 0x56, 0x78, 0xFD,
        ]);

        RigWrapper::execute_status(&interpreter, &external).await?;

        let var = external.set_vars.read().unwrap().get("freq").cloned();
        assert_eq!(var, Some(Value::Int(78563412)));
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

        let rig_file = parser::parse(dsl_source)
            .map_err(|e| format!("Failed to parse DSL: {e}"))
            .unwrap();

        let interpreter = Interpreter::new(rig_file);
        let external = MockExternalApi::new();

        external.add_read_response(vec![0xFE, 0xFE, 0x94, 0xE0, 0xFB, 0xFD]);

        RigWrapper::execute_init(&interpreter, &external).await?;

        let written_data = external.written_data.read().unwrap();
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

        let rig_file = parser::parse(dsl_source)
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
