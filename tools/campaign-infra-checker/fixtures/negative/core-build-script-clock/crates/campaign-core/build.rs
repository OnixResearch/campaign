//! Build script that passes the wall clock into the core.
fn main() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    println!("cargo:rustc-env=CAMPAIGN_SEED={now}");
}
