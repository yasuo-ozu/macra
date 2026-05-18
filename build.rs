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

    // On Windows, also build the RUSTC_WRAPPER binary
    if cfg!(target_os = "windows") {
        let wrapper_target_dir = Path::new(&out_dir).join("macra-rustc-wrapper-target");

        let status = Command::new(&cargo)
            .arg("build")
            .arg("-p")
            .arg("macra-rustc-wrapper")
            .arg("--target-dir")
            .arg(&wrapper_target_dir)
            .args(if profile == "release" {
                vec!["--release"]
            } else {
                vec![]
            })
            .status()
            .expect("failed to run cargo build for macra-rustc-wrapper");

        if !status.success() {
            panic!("cargo build -p macra-rustc-wrapper failed");
        }

        let wrapper_name = "macra-rustc-wrapper.exe";
        let built_wrapper = wrapper_target_dir.join(&profile).join(wrapper_name);
        let wrapper_dest = Path::new(&out_dir).join(wrapper_name);

        std::fs::copy(&built_wrapper, &wrapper_dest).unwrap_or_else(|e| {
            panic!(
                "failed to copy {} -> {}: {}",
                built_wrapper.display(),
                wrapper_dest.display(),
                e
            )
        });
    }

    // Rerun when macra-hook sources change
    println!("cargo:rerun-if-changed=macra-hook/src");
    println!("cargo:rerun-if-changed=macra-hook/Cargo.toml");
    println!("cargo:rerun-if-changed=macra-hook/build.rs");
    // Rerun when macra-rustc-wrapper sources change
    println!("cargo:rerun-if-changed=macra-rustc-wrapper/src");
    println!("cargo:rerun-if-changed=macra-rustc-wrapper/Cargo.toml");
}
