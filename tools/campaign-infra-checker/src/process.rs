//! Process-state capture for the checker command-line shell.

#[cfg(test)]
mod controls;

#[allow(
    unknown_lints,
    reason = "campaign-infra-checker::process uses an Octet-only lint during plain Rust builds"
)]
#[allow(
    ambient_env,
    reason = "campaign-infra-checker::process owns the single CLI process-state capture boundary"
)]
pub(crate) fn capture_request() -> Result<crate::model::CheckRequest, crate::failure::CheckError> {
    let inputs = crate::model::ProcessInputs {
        args: std::env::args_os().collect(),
        cairn_source_root: std::env::var_os(crate::policy::CAIRN_SOURCE_ROOT_ENV),
        choregraph_history_source: std::env::var_os(crate::policy::CHOREGRAPH_HISTORY_SOURCE_ENV),
    };
    inputs_to_request(inputs)
}

fn inputs_to_request(
    inputs: crate::model::ProcessInputs,
) -> Result<crate::model::CheckRequest, crate::failure::CheckError> {
    if inputs.args.len() != crate::policy::EXPECTED_ARG_COUNT {
        return Err(crate::failure::CheckError::usage(crate::policy::USAGE_TEXT));
    }
    assert_eq!(
        inputs.args.len(),
        crate::policy::EXPECTED_ARG_COUNT,
        "argument admission must enforce the exact CLI shape"
    );
    if inputs.args[crate::policy::ROOT_VALUE_ARG_INDEX].is_empty() {
        return Err(crate::failure::CheckError::usage(
            "the repository root must not be empty",
        ));
    }
    assert!(
        !inputs.args[crate::policy::ROOT_VALUE_ARG_INDEX].is_empty(),
        "root admission must supply a native path before file loading"
    );

    if inputs.args[crate::policy::ROOT_FLAG_ARG_INDEX]
        != std::ffi::OsStr::new(crate::policy::ROOT_FLAG)
    {
        return Err(crate::failure::CheckError::usage(format!(
            "expected {} before the root path",
            crate::policy::ROOT_FLAG
        )));
    }

    Ok(crate::model::CheckRequest {
        command: parse_command(&inputs.args[crate::policy::COMMAND_ARG_INDEX])?,
        root: std::path::PathBuf::from(&inputs.args[crate::policy::ROOT_VALUE_ARG_INDEX]),
        cairn_source_root: inputs.cairn_source_root.map(std::path::PathBuf::from),
        choregraph_history_source: inputs
            .choregraph_history_source
            .map(std::path::PathBuf::from),
    })
}

fn parse_command(
    command: &std::ffi::OsStr,
) -> Result<crate::model::CheckCommand, crate::failure::CheckError> {
    if command == std::ffi::OsStr::new(crate::policy::COMMAND_ALL) {
        return Ok(crate::model::CheckCommand::All);
    }
    if command == std::ffi::OsStr::new(crate::policy::COMMAND_BOUNDARIES) {
        return Ok(crate::model::CheckCommand::Boundaries);
    }
    if command == std::ffi::OsStr::new(crate::policy::COMMAND_CAIRN_POLICY_SOURCE) {
        return Ok(crate::model::CheckCommand::CairnPolicySource);
    }
    Err(crate::failure::CheckError::usage(format!(
        "unknown command for campaign-infra-checker: {}",
        command.to_string_lossy()
    )))
}

#[cfg(test)]
mod tests {
    const PROGRAM_NAME: &str = "campaign-infra-checker";
    const ROOT_PATH: &str = "/tmp/campaign";
    const INVALID_UTF8_BYTE: u8 = 0xFF;

    #[test]
    fn accepts_valid_os_arguments() -> Result<(), crate::failure::CheckError> {
        let inputs = crate::model::ProcessInputs {
            args: vec![
                std::ffi::OsString::from(PROGRAM_NAME),
                std::ffi::OsString::from(crate::policy::COMMAND_ALL),
                std::ffi::OsString::from(crate::policy::ROOT_FLAG),
                std::ffi::OsString::from(ROOT_PATH),
            ],
            cairn_source_root: None,
            choregraph_history_source: None,
        };

        let request = super::inputs_to_request(inputs);

        assert!(request.is_ok());
        let request = request?;
        assert_eq!(request.command, crate::model::CheckCommand::All);
        assert_eq!(request.root, std::path::PathBuf::from(ROOT_PATH));
        Ok(())
    }

    #[test]
    fn rejects_invalid_utf8_command_without_panicking() {
        #[cfg(unix)]
        {
            let invalid_command =
                <std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![
                    INVALID_UTF8_BYTE,
                ]);
            let inputs = crate::model::ProcessInputs {
                args: vec![
                    std::ffi::OsString::from(PROGRAM_NAME),
                    invalid_command,
                    std::ffi::OsString::from(crate::policy::ROOT_FLAG),
                    std::ffi::OsString::from(ROOT_PATH),
                ],
                cairn_source_root: None,
                choregraph_history_source: None,
            };

            let result = super::inputs_to_request(inputs);

            assert!(result.is_err());
        }
    }
}
