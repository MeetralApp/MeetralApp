use anyhow::{Context, Result};

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "com.meetral.desktop";

#[cfg(windows)]
pub fn encrypt(plaintext: &str) -> Result<Vec<u8>> {
    encrypt_for_account("gemini_api_key", plaintext)
}

#[cfg(windows)]
pub fn decrypt(ciphertext: &[u8]) -> Result<String> {
    decrypt_for_account("gemini_api_key", ciphertext)
}

#[cfg(windows)]
pub fn encrypt_for_account(_account: &str, plaintext: &str) -> Result<Vec<u8>> {
    use windows_dpapi::{encrypt_data, Scope};

    encrypt_data(plaintext.as_bytes(), Scope::User, None)
        .map_err(|e| anyhow::anyhow!("encrypt: {e}"))
}

#[cfg(windows)]
pub fn decrypt_for_account(_account: &str, ciphertext: &[u8]) -> Result<String> {
    use windows_dpapi::{decrypt_data, Scope};

    let bytes =
        decrypt_data(ciphertext, Scope::User, None).map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
    String::from_utf8(bytes).context("decrypted API key is not valid UTF-8")
}

#[cfg(target_os = "macos")]
pub fn encrypt(plaintext: &str) -> Result<Vec<u8>> {
    encrypt_for_account("gemini_api_key", plaintext)
}

#[cfg(target_os = "macos")]
pub fn decrypt(ciphertext: &[u8]) -> Result<String> {
    decrypt_for_account("gemini_api_key", ciphertext)
}

#[cfg(target_os = "macos")]
pub fn encrypt_for_account(account: &str, plaintext: &str) -> Result<Vec<u8>> {
    use keyring::Entry;

    let entry = Entry::new(KEYCHAIN_SERVICE, account)
        .map_err(|e| anyhow::anyhow!("keychain entry: {e}"))?;
    entry
        .set_password(plaintext)
        .map_err(|e| anyhow::anyhow!("keychain set: {e}"))?;
    Ok(format!("keychain:{account}").into_bytes())
}

#[cfg(target_os = "macos")]
pub fn decrypt_for_account(account: &str, ciphertext: &[u8]) -> Result<String> {
    use keyring::Entry;

    let marker = format!("keychain:{account}");
    if ciphertext == marker.as_bytes() || ciphertext == b"keychain" && account == "gemini_api_key" {
        let entry = Entry::new(KEYCHAIN_SERVICE, account)
            .map_err(|e| anyhow::anyhow!("keychain entry: {e}"))?;
        return entry
            .get_password()
            .map_err(|e| anyhow::anyhow!("keychain get: {e}"));
    }

    String::from_utf8(ciphertext.to_vec()).context("decrypted API key is not valid UTF-8")
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn encrypt(plaintext: &str) -> Result<Vec<u8>> {
    encrypt_for_account("gemini_api_key", plaintext)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn decrypt(ciphertext: &[u8]) -> Result<String> {
    decrypt_for_account("gemini_api_key", ciphertext)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn encrypt_for_account(_account: &str, plaintext: &str) -> Result<Vec<u8>> {
    Ok(plaintext.as_bytes().to_vec())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn decrypt_for_account(_account: &str, ciphertext: &[u8]) -> Result<String> {
    String::from_utf8(ciphertext.to_vec()).context("decrypted API key is not valid UTF-8")
}

/// Best-effort secret delete for custom profiles. On macOS the password
/// lives in the real keychain and must be deleted explicitly; on Windows the
/// "secret" is the DPAPI ciphertext inside config.json, so dropping the field
/// already deletes it (no-op here).
#[cfg(target_os = "macos")]
pub fn delete_for_account(account: &str) -> Result<()> {
    use keyring::Entry;

    let entry = Entry::new(KEYCHAIN_SERVICE, account)
        .map_err(|e| anyhow::anyhow!("keychain entry: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("keychain delete: {e}")),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn delete_for_account(_account: &str) -> Result<()> {
    Ok(())
}
