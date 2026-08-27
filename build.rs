use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this script only if these files change
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=about.hbs");
    println!("cargo:rerun-if-changed=about.toml");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(out_dir).join("licenses.txt");

    // Execute cargo-about
    let output = Command::new("cargo")
        .args(["about", "generate", "about.hbs"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            // Write the generated text to the OUT_DIR
            fs::write(&dest_path, out.stdout).unwrap();
        }
        _ => {
            // Provide a fallback so the build doesn't hard-crash if cargo-about is missing
            fs::write(
                &dest_path,
                "License generation failed or cargo-about is not installed.",
            )
            .unwrap();
        }
    }
}
