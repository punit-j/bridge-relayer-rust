pub fn read_private_key_from_file(absolute_path: &str) -> String {
    let data = std::fs::read_to_string(absolute_path).expect("Unable to read file");
    let mut res: serde_json::Value = serde_json::from_str(&data).expect("Unable to parse");
    res["private_key"].take().to_string().replace("\"", "")
}
