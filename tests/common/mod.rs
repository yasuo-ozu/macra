#[derive(Debug)]
pub struct ExpansionBlock {
    pub caller: String,
    pub input: String,
    pub output: String,
}

/// Parse the `--show-expansion` stdout into structured blocks.
///
/// Each block has the format:
///   == caller ==
///   input (may be empty / multi-line)
///   ---
///   output (may be multi-line)
pub fn parse_expansion_blocks(stdout: &str) -> Vec<ExpansionBlock> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(caller) = line.strip_prefix("== ").and_then(|s| s.strip_suffix(" ==")) {
            let caller = caller.to_string();
            i += 1;

            let mut input_lines = Vec::new();
            while i < lines.len() && lines[i] != "---" {
                input_lines.push(lines[i]);
                i += 1;
            }

            if i < lines.len() && lines[i] == "---" {
                i += 1;
            }

            let mut output_lines = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if l.starts_with("== ") && l.ends_with(" ==") {
                    break;
                }
                output_lines.push(l);
                i += 1;
            }

            while output_lines.last().is_some_and(|l| l.is_empty()) {
                output_lines.pop();
            }

            blocks.push(ExpansionBlock {
                caller,
                input: input_lines.join("\n"),
                output: output_lines.join("\n"),
            });
        } else {
            i += 1;
        }
    }

    blocks
}

/// Find all blocks with a given caller name.
pub fn find_blocks<'a>(blocks: &'a [ExpansionBlock], caller: &str) -> Vec<&'a ExpansionBlock> {
    blocks.iter().filter(|b| b.caller == caller).collect()
}
