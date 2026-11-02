//! The iOS app's Microsoft tokens, in its own Keychain items (docs/ios.md): the store
//! `launcher/src/account.rs` signs in through, as `android/secure_store.rs` is on Android.
use crate::account::{SERVICE, SecretStore};

pub(crate) struct IosStore;

impl SecretStore for IosStore {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        match keyring::Entry::new(SERVICE, key).and_then(|entry| entry.get_password()) {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, key: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|e| e.to_string())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, key).and_then(|entry| entry.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn persistent(&self) -> bool {
        true
    }
}
