/// This is an add on for the RustlsConnector.
///
/// This module implements a custom ServerCertVerifyer which does not perform
/// any certificate validation at all. It accepts any certificate and does
/// not perform host name validation. The implemented behaviour is highly unsecure!
///
/// WARNING - USE WITH CARE - THIS IS A POSSIBLE SECURITY RISK
///
/// generate_non_verifying_config() offers any easy way to get a unsafe
/// ClientConfig for use with the EppClient:
///
/// example:
///
///    // Create an instance of EppClient
///    let timeout = Duration::from_secs(5);
///    let config = dangerous::generate_non_verifying_config().expect("CONFIG");
///    let con = RustlsConnector::new_with_clientconfig((hostname.to_owned(), port.try_into().unwrap()), config).await.expect("CONNECTOR");
///    let mut client = EppClient::new(con, "tld".to_string(), timeout).await.expect("CLIENT");
///
///


use crate::error::Error;
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use rustls_native_certs::CertificateResult;
use std::sync::Arc;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::rustls::client::danger::*; 
use tokio_rustls::rustls::DigitallySignedStruct;
use tokio_rustls::rustls::SignatureScheme;


/// generate a ClientConfig which utilizes NonVerifyingCertVerifier for certificate validation.
/// WARNING: highly unsafe !!!
///
pub fn generate_non_verifying_config(
) -> Result<ClientConfig, Error> {
    let mut roots = RootCertStore::empty();
    let CertificateResult {
        certs, mut errors, ..
    } = rustls_native_certs::load_native_certs();
    if let Some(err) = errors.pop() {
        return Err(Error::Other(err.into()));
    }

    for cert in certs {
        roots.add(cert).map_err(|err| {
            Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    }

    let mut config = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    config.dangerous().set_certificate_verifier(Arc::new(NonVerifyingCertVerifier {}));
    Ok( config )
}


/// NonVerifyingCertVerifier can be used as an alternative to the default 
/// ServerCertVerifyer.
/// This specific implementation does not perform any validations.
/// The verificators just return its specific verified assertion.
/// This behaviour is higly unsecure. Do use with consideration and care.
///
#[derive(Debug)]
pub struct NonVerifyingCertVerifier {}

impl ServerCertVerifier for NonVerifyingCertVerifier {
    // Required methods
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        // unconditional success
        Ok(ServerCertVerified::assertion())
    }


    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        // unconditional success
      Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        // unconditional success
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // add all possible algos
        vec![SignatureScheme::RSA_PKCS1_SHA1,
             SignatureScheme::ECDSA_SHA1_Legacy,
             SignatureScheme::RSA_PKCS1_SHA256,
             SignatureScheme::ECDSA_NISTP256_SHA256,
             SignatureScheme::RSA_PKCS1_SHA384,
             SignatureScheme::ECDSA_NISTP384_SHA384,
             SignatureScheme::RSA_PKCS1_SHA512,
             SignatureScheme::ECDSA_NISTP521_SHA512,
             SignatureScheme::RSA_PSS_SHA256,
             SignatureScheme::RSA_PSS_SHA384,
             SignatureScheme::RSA_PSS_SHA512,
             SignatureScheme::ED25519,
             SignatureScheme::ED448]
    }
}
