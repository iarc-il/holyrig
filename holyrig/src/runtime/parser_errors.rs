#[derive(Debug, Clone)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl SourcePosition {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorLevel {
    #[default]
    Normal,
    Detailed,
    Verbose,
}

#[derive(Debug)]
pub struct ParseError {
    pub position: SourcePosition,
    pub error_type: Box<ParseErrorType>,
    pub source: String,
    pub level: ErrorLevel,
}

#[derive(Debug)]
pub enum ParseErrorType {
    Tokenization {
        message: String,
        context: String,
    },
    Syntax {
        expected: Vec<String>,
        found: Option<String>,
        context: String,
        peg_error: Option<String>,
        user_friendly_message: Option<String>,
    },
    Semantic {
        message: String,
        suggestion: Option<String>,
    },
}

impl ParseError {
    pub fn with_level(mut self, level: ErrorLevel) -> Self {
        self.level = level;
        self
    }

    fn format_context(&self, position: &SourcePosition) -> String {
        let lines: Vec<&str> = self.source.lines().collect();
        let line_idx = position.line.saturating_sub(1);

        let start_line = line_idx.saturating_sub(2);
        let end_line = (line_idx + 3).min(lines.len());

        let mut context = String::new();
        let line_num_width = (end_line.checked_ilog10().unwrap_or(0) + 1) as usize;
        for (i, line) in lines[start_line..end_line].iter().enumerate() {
            let current_line_num = start_line + i + 1;
            if current_line_num == position.line {
                context.push_str(&format!(" → {current_line_num:line_num_width$} | {line}\n"));
                context.push_str(&format!(
                    "   {} | {}{}\n",
                    " ".repeat(line_num_width),
                    " ".repeat(position.column.saturating_sub(1)),
                    "^"
                ));
            } else {
                context.push_str(&format!("   {current_line_num:line_num_width$} | {line}\n"));
            }
        }
        context
    }

    fn get_user_friendly_syntax_message(
        &self,
        expected: &[String],
        found: &Option<String>,
    ) -> String {
        if let Some(found_token) = found {
            if found_token.contains("ParenOpen")
                && expected
                    .iter()
                    .any(|e| e.contains("decimalnumber") || e.contains("id"))
            {
                return "Arithmetic expressions are not supported here. Use simple values like numbers, variables, or method calls instead.".to_string();
            }

            if found_token.contains("BraceOpen") && expected.iter().any(|e| e.contains("semicolon"))
            {
                return "Missing semicolon. Each statement must end with ';'".to_string();
            }

            if found_token.contains("Id") && expected.iter().any(|e| e.contains("braceclose")) {
                return "Missing comma between enum variants or list items.".to_string();
            }
        }

        let simplified_expected: Vec<String> = expected
            .iter()
            .filter_map(|e| {
                if e.contains("decimalnumber") {
                    Some("number".to_string())
                } else if e.contains("hexnumber") {
                    Some("hex number (0xFF)".to_string())
                } else if e.contains("id") {
                    Some("identifier".to_string())
                } else if e.contains("str") {
                    Some("string".to_string())
                } else if e.contains("semicolon") {
                    Some("semicolon (;)".to_string())
                } else if e.contains("comma") {
                    Some("comma (,)".to_string())
                } else if e.contains("braceopen") {
                    Some("opening brace ({)".to_string())
                } else if e.contains("braceclose") {
                    Some("closing brace (})".to_string())
                } else if e.contains("impl") {
                    Some("'impl' keyword".to_string())
                } else if e.contains("enum") {
                    Some("enum definition".to_string())
                } else if e.contains("fn") {
                    Some("function definition".to_string())
                } else {
                    None
                }
            })
            .collect();

        if !simplified_expected.is_empty() {
            format!("Expected {}", simplified_expected.join(" or "))
        } else {
            format!("Unexpected syntax, Expected tokens: {expected:?}")
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error_type.as_ref() {
            ParseErrorType::Tokenization { message, context } => match self.level {
                ErrorLevel::Normal => {
                    writeln!(
                        f,
                        "Invalid character at line {}, column {}",
                        self.position.line, self.position.column
                    )?;
                    writeln!(f, "{}", self.format_context(&self.position))?;
                }
                ErrorLevel::Detailed | ErrorLevel::Verbose => {
                    writeln!(
                        f,
                        "Tokenization error at line {}, column {}:",
                        self.position.line, self.position.column
                    )?;
                    writeln!(f, "{message}")?;
                    writeln!(f, "{}", self.format_context(&self.position))?;
                    if !context.is_empty() && self.level == ErrorLevel::Verbose {
                        write!(f, "Context: {context}")?;
                    }
                }
            },
            ParseErrorType::Syntax {
                expected,
                found,
                context,
                peg_error,
                user_friendly_message,
            } => match self.level {
                ErrorLevel::Normal => {
                    writeln!(
                        f,
                        "Syntax error at line {}, column {}:",
                        self.position.line, self.position.column
                    )?;

                    if let Some(friendly_msg) = user_friendly_message {
                        writeln!(f, "{friendly_msg}")?;
                    } else {
                        let friendly_msg = self.get_user_friendly_syntax_message(expected, found);
                        writeln!(f, "{friendly_msg}")?;
                    }

                    write!(f, "{}", self.format_context(&self.position))?;
                }
                ErrorLevel::Detailed => {
                    writeln!(
                        f,
                        "Syntax error at line {}, column {}:",
                        self.position.line, self.position.column
                    )?;

                    if let Some(friendly_msg) = user_friendly_message {
                        writeln!(f, "{friendly_msg}")?;
                    } else {
                        let friendly_msg = self.get_user_friendly_syntax_message(expected, found);
                        writeln!(f, "{friendly_msg}")?;
                    }

                    if let Some(found_token) = found {
                        writeln!(f, "Found: {found_token}")?;
                    }
                    writeln!(f, "Expected: {}", expected.join(", "))?;
                    write!(f, "{}", self.format_context(&self.position))?;
                }
                ErrorLevel::Verbose => {
                    writeln!(
                        f,
                        "Syntax error at line {}, column {}:",
                        self.position.line, self.position.column
                    )?;

                    if let Some(found_token) = found {
                        writeln!(
                            f,
                            "Found '{}', but expected one of: {}",
                            found_token,
                            expected.join(", ")
                        )?;
                    } else {
                        writeln!(f, "Expected one of: {}", expected.join(", "))?;
                    }

                    writeln!(f, "{}", self.format_context(&self.position))?;

                    if !context.is_empty() {
                        writeln!(f, "Context: {context}")?;
                    }

                    if let Some(peg_err) = peg_error {
                        writeln!(f, "PEG Error: {peg_err}")?;
                    }
                }
            },
            ParseErrorType::Semantic {
                message,
                suggestion,
            } => {
                writeln!(
                    f,
                    "Error at line {}, column {}:",
                    self.position.line, self.position.column
                )?;
                writeln!(f, "{message}")?;
                writeln!(f, "{}", self.format_context(&self.position))?;

                if let Some(suggestion) = suggestion {
                    writeln!(f, "Suggestion: {suggestion}")?;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

pub(crate) fn calculate_position(source: &str, offset: usize) -> SourcePosition {
    let mut line = 1;
    let mut column = 1;

    for (i, char) in source.char_indices() {
        if i >= offset {
            break;
        }
        if char == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    SourcePosition::new(line, column, offset)
}
