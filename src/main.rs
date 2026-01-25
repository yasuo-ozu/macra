use macra::parse_normal::parse_normal_output;

fn main() {
    let output = std::fs::read_to_string("/tmp/coinduction2_output.txt")
        .expect("Failed to read file");

    let source = parse_normal_output(&output);

    println!("=== Parsed source code ({} lines) ===", source.lines().count());
    // Print first 50 lines
    for (i, line) in source.lines().take(50).enumerate() {
        println!("{:4}: {}", i + 1, line);
    }
    println!("...");
    println!("=== Last 10 lines ===");
    let lines: Vec<_> = source.lines().collect();
    for (i, line) in lines.iter().rev().take(10).rev().enumerate() {
        println!("{:4}: {}", lines.len() - 9 + i, line);
    }
}
