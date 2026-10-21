//! Argument rejection controls must return errors without panics.

fn arguments(root: &str) -> crate::model::ProcessInputs {
    crate::model::ProcessInputs {
        args: ["checker", "all", "--root", root]
            .map(std::ffi::OsString::from)
            .to_vec(),
        cairn_source_root: None,
        choregraph_history_source: None,
    }
}

#[test]
fn rejects_empty_root() {
    let result = super::inputs_to_request(arguments(""));
    assert!(result.is_err(), "an empty root must not reach file loading");
}

#[test]
fn accepts_literal_flag_as_native_path() -> Result<(), crate::failure::CheckError> {
    let request = super::inputs_to_request(arguments("--root"))?;
    assert_eq!(request.root, std::path::Path::new("--root"));
    Ok(())
}

#[test]
fn rejects_wrong_root_flag() {
    let mut inputs = arguments(".");
    inputs.args[crate::policy::ROOT_FLAG_ARG_INDEX] = std::ffi::OsString::from("--other");
    assert!(super::inputs_to_request(inputs).is_err());
}

#[test]
fn rejects_missing_arguments() {
    let mut inputs = arguments(".");
    inputs.args.clear();
    assert!(super::inputs_to_request(inputs).is_err());
}
