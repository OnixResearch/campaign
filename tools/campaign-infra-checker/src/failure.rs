//! Failure type for checker decisions and shell failures.

use std::error::Error;

#[derive(Debug)]
pub(crate) struct CheckError {
    exit_code: i32,
    rule: Option<crate::rules::Rule>,
    message: String,
}

impl CheckError {
    pub(crate) fn io(error: &std::io::Error) -> Self {
        Self::failed(error.to_string())
    }

    pub(crate) fn failed(message: impl Into<String>) -> Self {
        Self {
            exit_code: crate::policy::FAILURE_EXIT,
            rule: None,
            message: message.into(),
        }
    }

    pub(crate) fn violation(rule: crate::rules::Rule, message: impl AsRef<str>) -> Self {
        Self {
            exit_code: crate::policy::FAILURE_EXIT,
            rule: Some(rule),
            message: message.as_ref().to_owned(),
        }
    }

    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: crate::policy::USAGE_EXIT,
            rule: None,
            message: message.into(),
        }
    }

    pub(crate) const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    #[cfg(test)]
    pub(crate) const fn rule(&self) -> Option<crate::rules::Rule> {
        self.rule
    }

    #[cfg(test)]
    pub(crate) const fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(rule) = self.rule {
            write!(formatter, "[{}] ", rule.spec_id())?;
        }
        formatter.write_str(&self.message)
    }
}

impl Error for CheckError {}
