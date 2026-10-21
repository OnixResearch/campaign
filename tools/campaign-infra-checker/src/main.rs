//! Repository checks for source provenance, consumer pilot evidence, the Choregraph pin,
//! dependency policy, source boundaries, and vendored policy files.

mod consumers;
mod dependencies;
mod failure;
mod fsio;
mod inventory;
mod model;
mod patterns;
mod pins;
mod policy;
mod process;
mod rules;
mod rustscope;
mod syntax;
mod validation;

fn main() {
    let result = process::capture_request().and_then(|request| fsio::run_request(&request));
    match result {
        Ok(()) => std::process::exit(policy::SUCCESS_EXIT),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(error.exit_code());
        }
    }
}
