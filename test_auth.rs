use std::path::PathBuf;
fn main() {
    let path = PathBuf::from("E:\\AI_field\\hermes-rust-win\\test_home\\credentials.yaml");
    println!("Path: {:?}", path);
    println!("Exists: {}", path.exists());
    if let Ok(content) = std::fs::read_to_string(&path) {
        println!("Content: {}", content);
    }
}
