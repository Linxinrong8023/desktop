//! Minisign adapter for the Tauri updater's trust configuration.

use super::UpdateError;
use super::artifact_store::ArtifactVerifier;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use ora_utils::hash::sha256_reader;
use std::io::Cursor;
use tauri::AppHandle;

/// Verifies persisted package bytes with the same public key configured for Tauri updater.
pub(super) struct UpdateVerifier {
    public_key: PublicKey,
    trust_root_fingerprint: String,
}

impl UpdateVerifier {
    /// Reads and decodes the updater public key from the application's single Tauri configuration.
    pub(super) fn from_app(app: &AppHandle) -> Result<Self, UpdateError> {
        let encoded = app
            .config()
            .plugins
            .0
            .get("updater")
            .and_then(|config| config.get("pubkey"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                UpdateError::TrustConfiguration("plugins.updater.pubkey is missing".to_owned())
            })?;
        let decoded = STANDARD
            .decode(encoded)
            .map_err(|error| UpdateError::TrustConfiguration(error.to_string()))?;
        let decoded = std::str::from_utf8(&decoded)
            .map_err(|error| UpdateError::TrustConfiguration(error.to_string()))?;
        let public_key = PublicKey::decode(decoded)
            .map_err(|error| UpdateError::TrustConfiguration(error.to_string()))?;
        let trust_root_fingerprint =
            sha256_reader(Cursor::new(encoded.as_bytes())).map_err(UpdateError::CacheRead)?;
        Ok(Self {
            public_key,
            trust_root_fingerprint,
        })
    }
}

impl ArtifactVerifier for UpdateVerifier {
    /// Identifies the configured updater key without exposing it in the cache record.
    fn trust_root_fingerprint(&self) -> &str {
        &self.trust_root_fingerprint
    }

    /// Repeats Tauri's Minisign verification for package bytes restored from disk.
    fn verify(&self, bytes: &[u8], encoded_signature: &str) -> bool {
        let Ok(decoded) = STANDARD.decode(encoded_signature) else {
            return false;
        };
        let Ok(decoded) = std::str::from_utf8(&decoded) else {
            return false;
        };
        let Ok(signature) = Signature::decode(decoded) else {
            return false;
        };
        self.public_key.verify(bytes, &signature, true).is_ok()
    }
}
