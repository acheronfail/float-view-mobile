use std::{env, fs, path::PathBuf};
fn main() {
    let source = PathBuf::from("../../frontend/build/index.html");
    println!("cargo:rerun-if-changed={}", source.display());
    let bytes =
        fs::read(&source).expect("Build the shared frontend first: cd frontend && npm run build");
    assert!(
        !bytes.is_empty() && bytes.len() <= 8 * 1024 * 1024,
        "Invalid frontend artifact"
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("index.html"),
        bytes,
    )
    .unwrap();
}
