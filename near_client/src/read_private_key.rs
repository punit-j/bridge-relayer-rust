use near_crypto;
use std::str::FromStr;

pub fn read_private_key_from_file(
    absolute_path: &str,
) -> Result<near_crypto::InMemorySigner, String> {
    let data = std::fs::read_to_string(absolute_path)
        .map_err(|e| format!("Unable to read file {}: {}", absolute_path, e))?;
    let res: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| format!("Unable to parse {}: {}", absolute_path, e))?;

    let private_key = res["private_key"].to_string().replace('\"', "");
    let private_key =
        near_crypto::SecretKey::from_str(private_key.as_str()).map_err(|e| e.to_string())?;

    let account_id = res["account_id"].to_string().replace('\"', "");
    let account_id = near_primitives::types::AccountId::from_str(account_id.as_str())
        .map_err(|e| e.to_string())?;

    Ok(near_crypto::InMemorySigner::from_secret_key(
        account_id,
        private_key,
    ))
}

#[cfg(test)]
pub mod tests {
    use crate::read_private_key::read_private_key_from_file;
    use std::path::Path;
    use std::ffi::OsStr;
    use crate::test_utils::get_near_signer;

    #[tokio::test]
    async fn smoke_read_private_key_from_file_test() {
        get_near_signer()
    }
}