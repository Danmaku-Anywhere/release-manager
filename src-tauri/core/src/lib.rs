pub mod active;
pub mod cache;
pub mod error;
pub mod github;
pub mod manager;
pub mod menu;
pub mod store;
pub mod train;
pub mod types;

pub fn ensure_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}
