fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("BUILD_VERSION").ok())
        .unwrap_or_else(|| {
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into())
        });

    println!("cargo:rustc-env=BUILD_VERSION={version}");
}
