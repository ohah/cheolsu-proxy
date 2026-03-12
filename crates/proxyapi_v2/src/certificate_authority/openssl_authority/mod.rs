mod cert_gen;
#[cfg(feature = "openssl-ca")]
mod openssl_context;
#[cfg(test)]
mod tests;
mod trait_impl;

use crate::certificate_authority::CACHE_TTL;
use http::uri::Authority;
use moka::future::Cache;
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    x509::X509,
};
use std::{sync::Arc, time::Duration};
use tokio_rustls::rustls::{
    ServerConfig,
    crypto::CryptoProvider,
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
};

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Certificates are generated using
/// the `openssl` crate.
///
/// # Examples
///
/// ```rust
/// use proxyapi_v2::{
///     certificate_authority::OpensslAuthority,
///     openssl::{hash::MessageDigest, pkey::PKey, x509::X509},
///     rustls::crypto::aws_lc_rs,
/// };
///
/// let private_key_bytes: &[u8] = include_bytes!("../../../examples/ca/hudsucker.key");
/// let ca_cert_bytes: &[u8] = include_bytes!("../../../examples/ca/hudsucker.cer");
/// let private_key = PKey::private_key_from_pem(private_key_bytes).unwrap();
/// let ca_cert = X509::from_pem(ca_cert_bytes).unwrap();
///
/// let ca = OpensslAuthority::new(
///     private_key,
///     ca_cert,
///     MessageDigest::sha256(),
///     1_000,
///     aws_lc_rs::default_provider(),
/// );
/// ```
pub struct OpensslAuthority {
    pub(super) pkey: PKey<Private>,
    pub(super) private_key: PrivateKeyDer<'static>,
    pub(super) ca_cert: X509,
    /// DER 형태의 CA 인증서 (spawn_blocking 전달용, 생성자에서 미리 캐시)
    pub(super) ca_cert_der: Vec<u8>,
    /// DER 형태의 개인키 (spawn_blocking 전달용, 생성자에서 미리 캐시)
    pub(super) pkey_der: Vec<u8>,
    pub(super) hash: MessageDigest,
    pub(super) cache: Cache<Authority, Arc<ServerConfig>>,
    #[cfg(feature = "openssl-ca")]
    pub(super) openssl_ctx_cache: Cache<Authority, Arc<openssl::ssl::SslContext>>,
    pub(super) provider: Arc<CryptoProvider>,
}

impl OpensslAuthority {
    /// Creates a new openssl authority.
    pub fn new(
        pkey: PKey<Private>,
        ca_cert: X509,
        hash: MessageDigest,
        cache_size: u64,
        provider: CryptoProvider,
    ) -> Self {
        let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(
            pkey.private_key_to_pkcs8()
                .expect("Failed to encode private key"),
        ));
        let ca_cert_der = ca_cert.to_der().expect("Failed to encode CA cert to DER");
        let pkey_der = pkey
            .private_key_to_der()
            .expect("Failed to encode private key to DER");

        Self {
            pkey,
            private_key,
            ca_cert,
            ca_cert_der,
            pkey_der,
            hash,
            cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(Duration::from_secs(CACHE_TTL))
                .build(),
            #[cfg(feature = "openssl-ca")]
            openssl_ctx_cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(Duration::from_secs(CACHE_TTL))
                .build(),
            provider: Arc::new(provider),
        }
    }
}
