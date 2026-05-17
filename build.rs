use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // Use a separate target-dir to avoid cargo lock contention
    let hook_target_dir = Path::new(&out_dir).join("macra-hook-target");

    let status = Command::new(&cargo)
        .arg("build")
        .arg("-p")
        .arg("macra-hook")
        .arg("--target-dir")
        .arg(&hook_target_dir)
        .args(if profile == "release" {
            vec!["--release"]
        } else {
            vec![]
        })
        .status()
        .expect("failed to run cargo build for macra-hook");

    if !status.success() {
        panic!("cargo build -p macra-hook failed");
    }

    // Determine the platform-specific library name
    let lib_name = if cfg!(target_os = "macos") {
        "libmacra_hook.dylib"
    } else if cfg!(target_os = "windows") {
        "macra_hook.dll"
    } else {
        "libmacra_hook.so"
    };

    let built_lib = hook_target_dir.join(&profile).join(lib_name);
    let dest = Path::new(&out_dir).join(lib_name);

    std::fs::copy(&built_lib, &dest).unwrap_or_else(|e| {
        panic!(
            "failed to copy {} -> {}: {}",
            built_lib.display(),
            dest.display(),
            e
        )
    });

    // Rerun when macra-hook sources change
    println!("cargo:rerun-if-changed=macra-hook/src");
    println!("cargo:rerun-if-changed=macra-hook/Cargo.toml");
    println!("cargo:rerun-if-changed=macra-hook/build.rs");
}
