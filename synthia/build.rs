use std::path::PathBuf;

fn main() {
    // Verify powertools binary exists at compile time
    // In a Cargo workspace, target directory is at workspace root
    let powertools_path = PathBuf::from("../target/release/powertools");

    if !powertools_path.exists() {
        panic!(
            "Powertools binary not found at {}. Please build it first:\n  cd ../powertools-cli && cargo build --release",
            powertools_path.display()
        );
    }

    println!("cargo:rerun-if-changed=../target/release/powertools");
    println!("cargo:rerun-if-changed=../powertools-cli/src");
}
