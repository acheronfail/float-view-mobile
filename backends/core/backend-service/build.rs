fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--symbolic-full-name", "HEAD"])
        .output()
    {
        let reference = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if reference.starts_with("refs/") {
            println!("cargo:rerun-if-changed=../../../.git/{reference}");
        }
    }
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=FLOAT_VIEW_BUILD_REVISION={revision}");
}
