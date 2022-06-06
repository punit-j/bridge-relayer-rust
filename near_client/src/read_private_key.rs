pub fn read_private_key_from_file(absolute_path: &str) -> String {
    let data = std::fs::read_to_string(absolute_path)
        .expect(format!("Unable to read file {}", absolute_path).as_str());
    let mut res: serde_json::Value =
        serde_json::from_str(&data).expect(format!("Unable to parse {}", absolute_path).as_str());
    res["private_key"].take().to_string().replace("\"", "")
}
