fn main() {
    let home = std::env::var("HERMES_HOME").unwrap_or_default();
    println!("HERMES_HOME env: {}", home);
    let path = std::path::PathBuf::from(&home).join("credentials.yaml");
    println!("Expected path: {:?}", path);
    println!("Path exists: {}", path.exists());
    if let Ok(content) = std::fs::read_to_string(&path) {
        println!("Content length: {}", content.len());
    }
}
