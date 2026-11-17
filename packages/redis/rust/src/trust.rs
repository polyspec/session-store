//! The authorities that the connections of TLS trust: those of the trust store of the machine, read once in a process
//! (SR-6.1b), or those of the file that a setting names, read once in a process for a path (SR-6.7).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use polyspec_session_store_core::Error;
use redis::TlsCertificates;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::CertificateDer;

/// The reads of the trust store of the machine in this process, which a test counts (SR-6.1b).
static READS: AtomicUsize = AtomicUsize::new(0);

/// The authorities that every client of TLS of the process trusts (SR-6.1b).
/// The authorities of the trust store as PEM text, once they were read; a read that failed leaves nothing, so that the
/// next connection reads again (SR-6.1b).
static READ: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// The authorities of the trust store of the machine as PEM text, read once in a process (SR-6.1b).
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

/// The authorities of the files that settings name, as PEM text beside their path, once they were read; a read that
/// failed leaves nothing, so that the next connection reads again (SR-6.7).
static FILES: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());

/// The authorities that the connections of TLS of a setting trust, as PEM text: those of the file `path` alone when
/// the setting names one, read once in a process for a path (SR-6.7), and else those of the trust store of the
/// machine (SR-6.1b).
pub(crate) fn authorities_of(path: Option<&str>) -> Result<Vec<u8>, Error> {
    let Some(path) = path else {
        return authorities();
    };
    let mut files = match FILES.lock() {
        Ok(held) => held,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some((_, pem)) = files.iter().find(|(read, _)| read == path) {
        return Ok(pem.clone());
    }
    let pem = file(path)?;
    files.push((path.to_owned(), pem.clone()));
    Ok(pem)
}

/// Reads the file of authorities `path` and gives its certificates as PEM text. A file that cannot be read or that
/// holds no PEM certificate is a setting that is not valid, and the text names the path, which is no secret (SR-6.7).
fn file(path: &str) -> Result<Vec<u8>, Error> {
    let invalid = |detail: String| Error::InvalidSetting { detail };
    let text =
        std::fs::read(path).map_err(|error| invalid(format!("the file of authorities {path} is not read: {error}")))?;
    let certificates = CertificateDer::pem_slice_iter(&text)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| invalid(format!("the file of authorities {path} is no PEM: {error}")))?;
    if certificates.is_empty() {
        return Err(invalid(format!("the file of authorities {path} holds no PEM certificate")));
    }
    pem(certificates.iter().map(AsRef::as_ref))
}

/// The authorities that every client of TLS of the process trusts (SR-6.1b).
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

    use polyspec_session_store_core::Error;

    use super::{authorities_of, pem};

    #[test]
    fn the_pem_of_the_authorities_reads_back_as_their_der() -> Result<(), Box<dyn std::error::Error>> {
        let ders: [&[u8]; 3] = [b"f", b"fo", &[0_u8; 100]];
        let text = pem(ders.iter().copied())?;
        let read = CertificateDer::pem_slice_iter(&text).collect::<Result<Vec<_>, _>>()?;
        assert_eq!(read.iter().map(AsRef::as_ref).collect::<Vec<&[u8]>>(), ders);
        assert!(pem(std::iter::empty())?.is_empty());
        Ok(())
    }

    /// SR-6.7: a setting that names a file trusts the authorities of that file alone, which is read once in a process
    /// for a path; a file that is missing or that holds no PEM certificate is a setting that is not valid, and the
    /// text names the path.
    #[test]
    fn the_authorities_of_a_file_are_those_of_the_file_alone() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("authorities.pem");
        let named = path.to_str().ok_or("the path of the test is no text")?;
        let ders: [&[u8]; 2] = [b"one authority", b"another authority"];
        std::fs::write(&path, pem(ders.iter().copied())?)?;
        let read = authorities_of(Some(named))?;
        let held = CertificateDer::pem_slice_iter(&read).collect::<Result<Vec<_>, _>>()?;
        assert_eq!(held.iter().map(AsRef::as_ref).collect::<Vec<&[u8]>>(), ders, "the file alone, not the trust store");
        // The path is read once in a process: the file is gone and the authorities stay.
        std::fs::remove_file(&path)?;
        assert_eq!(authorities_of(Some(named))?, read);

        let missing = directory.path().join("missing.pem");
        let missing = missing.to_str().ok_or("the path of the test is no text")?;
        let empty = directory.path().join("empty.pem");
        std::fs::write(&empty, "no certificate\n")?;
        let empty = empty.to_str().ok_or("the path of the test is no text")?;
        for path in [missing, empty] {
            match authorities_of(Some(path)) {
                Err(Error::InvalidSetting { detail }) => assert!(detail.contains(path), "{detail} names no {path}"),
                other => return Err(format!("the file {path} gives {other:?}").into()),
            }
        }
        Ok(())
    }
}
