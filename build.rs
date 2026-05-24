use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const NUM_TRAMPOLINES: usize = 256;

fn write_trampolines(out_dir: &Path) {
    let dest_path = out_dir.join("trampolines_generated.rs");
    let mut code = String::new();

    for i in 0..NUM_TRAMPOLINES {
        code.push_str(&format!(
            r#"
#[allow(unused)]
extern "C" fn trampoline_{i}(config: crate::types::BridgeConfig<'_>) -> crate::types::Buffer {{
    trampoline_impl({i}, config)
}}
"#
        ));
    }

    code.push_str(&format!(
        "\n#[allow(unused)]\npub const NUM_TRAMPOLINES: usize = {};\n\n",
        NUM_TRAMPOLINES
    ));
    code.push_str(
        "\n#[allow(unused)]\npub static TRAMPOLINE_FNS: [extern \"C\" fn(crate::types::BridgeConfig<'_>) -> crate::types::Buffer; NUM_TRAMPOLINES] = [\n",
    );
    for i in 0..NUM_TRAMPOLINES {
        code.push_str(&format!("    trampoline_{},\n", i));
    }
    code.push_str("];\n");

    fs::write(dest_path, code).expect("failed to write trampolines_generated.rs");
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    write_trampolines(Path::new(&out_dir));

    if env::var_os("MACRA_NESTED_BUILD").is_some() {
        let lib_name = if cfg!(target_os = "macos") {
            "libmacra_hook.dylib"
        } else if cfg!(target_os = "windows") {
            "macra_hook.dll"
        } else {
            "libmacra_hook.so"
        };
        let _ = fs::write(Path::new(&out_dir).join(lib_name), []);
        if cfg!(target_os = "windows") {
            let _ = fs::write(Path::new(&out_dir).join("macra-rustc-wrapper.exe"), []);
        }
        return;
    }

    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // Use a separate target-dir to avoid cargo lock contention
    let hook_target_dir = Path::new(&out_dir).join("macra-hook-target");

    // Determine the platform-specific library name
    let lib_name = if cfg!(target_os = "macos") {
        "libmacra_hook.dylib"
    } else if cfg!(target_os = "windows") {
        "macra_hook.dll"
    } else {
        "libmacra_hook.so"
    };

    let status = Command::new(&cargo)
        .arg("build")
        .arg("--example")
        .arg("macra-hook")
        .arg("--features")
        .arg("hook")
        .arg("--target-dir")
        .arg(&hook_target_dir)
        .args(if profile == "release" {
            vec!["--release"]
        } else {
            vec![]
        })
        .env("MACRA_NESTED_BUILD", "1")
        .status()
        .expect("failed to run cargo build for macra-hook example");

    if !status.success() {
        panic!("cargo build --example macra-hook failed");
    }

    let built_lib = hook_target_dir.join(&profile).join("examples").join(lib_name);
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
            .arg("--bin")
            .arg("macra-rustc-wrapper")
            .arg("--features")
            .arg("rustc-wrapper")
            .arg("--target-dir")
            .arg(&wrapper_target_dir)
            .env("MACRA_NESTED_BUILD", "1")
            .args(if profile == "release" {
                vec!["--release"]
            } else {
                vec![]
            })
            .status()
            .expect("failed to run cargo build for macra-rustc-wrapper");

        if !status.success() {
            panic!("cargo build --bin macra-rustc-wrapper failed");
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

    // Rerun when hook/wrapper sources change
    println!("cargo:rerun-if-changed=hook");
    println!("cargo:rerun-if-changed=rustc-wrapper");
}
