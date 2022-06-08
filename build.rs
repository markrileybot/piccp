use std::io::Result;

use version_rs::version;

fn main() -> Result<()> {
    println!(
        "cargo:rustc-env=VERSION_STRING={}",
        version(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_MANIFEST_DIR")
        )
    );
    Ok(())
}