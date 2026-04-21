use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AuthStore {
    #[serde(default)]
    credentials: Vec<ProviderCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderCredentials {
    provider: String,
    api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
}

fn main() {
    let path = std::path::PathBuf::from("E:\\AI_field\\hermes-rust-win\\test_home\\credentials.yaml");
    let content = std::fs::read_to_string(&path).unwrap();
    println!("Content: {}", content);
    let store: Result<AuthStore, _> = serde_yaml::from_str(&content);
    match store {
        Ok(s) => println!("Parsed: {:?}", s),
        Err(e) => println!("Parse error: {}", e),
    }
}
