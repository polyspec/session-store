//! The authorities of the trust store of the machine, read once in a process (SR-6.1a).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use polyspec_session_store_core::Error;
use redis::TlsCertificates;

/// The reads of the trust store of the machine in this process, which a test counts (SR-6.1a).
static READS: AtomicUsize = AtomicUsize::new(0);

/// The authorities that every client of TLS of the process trusts (SR-6.1a).
/// The authorities of the trust store as PEM text, once they were read; a read that failed leaves nothing, so that the
/// next connection reads again (SR-6.1a).
static READ: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// The authorities of the trust store of the machine as PEM text, read once in a process (SR-6.1a).
pub(crate) fn authorities() -> Result<Vec<u8>, Error> {
    let mut read_once = match READ.lock() {
        Ok(held) => held,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(pem) = read_once.as_ref() {
        return Ok(pem.clone());
    }
    let pem = read()?;
    *read_once = Some(pem.clone());
    Ok(pem)
}

/// The authorities that every client of TLS of the process trusts (SR-6.1a).
pub(crate) fn certificates(authorities: &[u8]) -> TlsCertificates {
    TlsCertificates { client_tls: None, root_cert: Some(authorities.to_vec()) }
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
        // A short last group is filled with zero bytes, whose characters the padding replaces.
        let mut group = [0_u8; 3];
        for (slot, byte) in group.iter_mut().zip(chunk) {
            *slot = *byte;
        }
        let [first, second, third] = group;
        let sixes = [first >> 2, ((first & 3) << 4) | (second >> 4), ((second & 15) << 2) | (third >> 6), third & 63];
        for (index, six) in sixes.into_iter().enumerate() {
            if index <= chunk.len() {
                text.push(char::from(ALPHABET[usize::from(six)]));
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
