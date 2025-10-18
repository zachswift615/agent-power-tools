// Quick test to verify cursor logic
fn main() {
    let mut input = String::from("I have a TODO app written in python/flask in ");
    let mut cursor_position = input.chars().count(); // Should be 45

    println!("Initial state:");
    println!("  input: '{}'", input);
    println!("  input.len() (bytes): {}", input.len());
    println!("  input.chars().count(): {}", input.chars().count());
    println!("  cursor_position: {}", cursor_position);

    // Simulate pasting a path character by character
    let paste_text = "/Users/zachswift/projects/agent-power-tools/.claude/agents";
    for c in paste_text.chars() {
        let byte_pos = char_to_byte_pos(&input, cursor_position);
        println!("  Inserting '{}' at char_pos={} (byte_pos={})", c, cursor_position, byte_pos);
        input.insert(byte_pos, c);
        cursor_position += 1;
    }

    println!("\nAfter paste:");
    println!("  input: '{}'", input);
    println!("  input.len() (bytes): {}", input.len());
    println!("  input.chars().count(): {}", input.chars().count());
    println!("  cursor_position: {}", cursor_position);

    // Try to move cursor right
    let char_len = input.chars().count();
    println!("\nTrying to move cursor right:");
    println!("  cursor_position: {}", cursor_position);
    println!("  char_len: {}", char_len);
    println!("  Can move right? {}", cursor_position < char_len);
}

fn char_to_byte_pos(input: &str, char_pos: usize) -> usize {
    input
        .char_indices()
        .nth(char_pos)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(input.len())
}
