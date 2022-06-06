#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub field: String,
}

impl Message {
    pub fn default() -> Self {
        Message {
            field: "Hello, World!".to_string(),
        }
    }
}
