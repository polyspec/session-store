//! The authorities of the trust store of the machine, read once in a process (SR-6.1a).

use std::sync::atomic::{AtomicUsize, Ordering};

use polyspec_session_store_core::Error;
use redis::TlsCertificates;

/// The reads of the trust store of the machine in this process, which a test counts (SR-6.1a).
static READS: AtomicUsize = AtomicUsize::new(0);

/// The authorities that every client of TLS of the process trusts (SR-6.1a).
#[allow(dead_code, reason = "SR-6.1a: no link takes the authorities yet")]
pub(crate) fn certificates() -> Result<TlsCertificates, Error> {
    Ok(TlsCertificates { client_tls: None, root_cert: Some(read()?) })
}

/// Reads the trust store of the machine and gives its authorities as PEM text; a trust store that cannot be read fails
/// the connection (SR-6.4).
fn read() -> Result<Vec<u8>, Error> {
    READS.fetch_add(1, Ordering::SeqCst);
    let found = rustls_native_certs::load_native_certs();
    if let Some(error) = found.errors.first() {
        return Err(Error::Connection { detail: format!("the trust store of the machine is not read: {error}") });
    }
    let mut pem = Vec::new();
    for certificate in &found.certs {
        pem.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");
        for line in base64(certificate.as_ref()).as_bytes().chunks(64) {
            pem.extend_from_slice(line);
            pem.push(b'\n');
        }
        pem.extend_from_slice(b"-----END CERTIFICATE-----\n");
    }
    Ok(pem)
}

/// The standard base64 text of bytes, with padding (RFC 4648 section 4), which PEM holds (RFC 7468).
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk.first().copied().unwrap_or_default();
        let second = chunk.get(1).copied().unwrap_or_default();
        let third = chunk.get(2).copied().unwrap_or_default();
        let group = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        for (index, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                let at = usize::try_from((group >> shift) & 63).unwrap_or_default();
                text.push(char::from(ALPHABET[at]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

#[cfg(test)]
pub(crate) fn reads() -> usize {
    READS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_pads_the_last_group() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
