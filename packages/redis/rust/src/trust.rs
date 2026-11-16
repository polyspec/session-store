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
    pem(found.certs.iter().map(AsRef::as_ref))
}

/// The PEM text of certificates of DER, one block `CERTIFICATE` each (RFC 7468), which `TlsCertificates` takes.
fn pem<'a>(certificates: impl Iterator<Item = &'a [u8]>) -> Result<Vec<u8>, Error> {
    let mut text = String::new();
    for certificate in certificates {
        let block =
            pem_rfc7468::encode_string("CERTIFICATE", pem_rfc7468::LineEnding::LF, certificate).map_err(|error| {
                Error::Connection { detail: format!("an authority of the trust store is not PEM: {error}") }
            })?;
        text.push_str(&block);
    }
    Ok(text.into_bytes())
}

#[cfg(test)]
pub(crate) fn reads() -> usize {
    READS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::CertificateDer;

    use super::pem;

    #[test]
    fn the_pem_of_the_authorities_reads_back_as_their_der() -> Result<(), Box<dyn std::error::Error>> {
        let ders: [&[u8]; 3] = [b"f", b"fo", &[0_u8; 100]];
        let text = pem(ders.iter().copied())?;
        let read = CertificateDer::pem_slice_iter(&text).collect::<Result<Vec<_>, _>>()?;
        assert_eq!(read.iter().map(AsRef::as_ref).collect::<Vec<&[u8]>>(), ders);
        assert!(pem(std::iter::empty())?.is_empty());
        Ok(())
    }
}
