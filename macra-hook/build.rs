use std::env;
use std::fs;
use std::path::Path;

const NUM_TRAMPOLINES: usize = 256;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("trampolines_generated.rs");

    let mut code = String::new();

    // Generate trampoline functions (one per proc macro entry)
    for i in 0..NUM_TRAMPOLINES {
        code.push_str(&format!(
            r#"
extern "C" fn trampoline_{i}(config: crate::types::BridgeConfig<'_>) -> crate::types::Buffer {{
    trampoline_impl({i}, config)
}}
"#,
            i = i
        ));
    }

    // Generate the trampoline array
    code.push_str(&format!(
        "\npub const NUM_TRAMPOLINES: usize = {};\n\n",
        NUM_TRAMPOLINES
    ));

    code.push_str(
        "pub static TRAMPOLINE_FNS: [extern \"C\" fn(crate::types::BridgeConfig<'_>) -> crate::types::Buffer; NUM_TRAMPOLINES] = [\n"
    );
    for i in 0..NUM_TRAMPOLINES {
        code.push_str(&format!("    trampoline_{},\n", i));
    }
    code.push_str("];\n");

    fs::write(&dest_path, code).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
