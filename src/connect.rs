use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
#[cfg(feature = "tokio-rustls")]
use tokio::net::lookup_host;
use tokio::net::TcpStream;
#[cfg(feature = "tokio-rustls")]
use tokio_rustls::client::TlsStream;
#[cfg(feature = "tokio-rustls")]
use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName};
#[cfg(feature = "tokio-rustls")]
use tokio_rustls::TlsConnector;
use tracing::info;

use crate::client::EppClient;
use crate::common::{Certificate, PrivateKey};
use crate::connection;
use crate::error::Error;

/// Connect to the specified `server` and `hostname` over TLS
///
/// The `registry` is used as a name in internal logging; `addr` provides the address to
/// connect to, `hostname` is sent as the TLS server name indication and `identity` provides
/// optional TLS client authentication (using) rustls as the TLS implementation.
/// The `timeout` limits the time spent on any underlying network operations.
///
/// This returns two halves, a cloneable client and the underlying connection.
///
/// Use connect_with_connector for passing a specific connector.
#[cfg(feature = "tokio-rustls")]
pub async fn connect(
    registry: String,
    server: (String, u16),
    identity: Option<(Vec<Certificate>, PrivateKey)>,
    timeout: Duration,
) -> Result<EppClient<RustlsConnector>, Error> {
    info!("Connecting to server: {:?}", server);

    let connector = RustlsConnector::new(server, identity).await?;
    EppClient::new(connector, registry, timeout).await
}

/// Connect to the specified `server` and `hostname` via the passed connector.
///
/// The `registry` is used as a name in internal logging; `addr` provides the address to
/// connect to, `hostname` is sent as the TLS server name indication and `identity` provides
/// optional TLS client authentication (using) rustls as the TLS implementation.
/// The `timeout` limits the time spent on any underlying network operations.
///
/// This returns two halves, a cloneable client and the underlying connection.
///
/// Use connect_with_connector for passing a specific connector.
pub async fn connect_with_connector<C>(
    connector: C,
    registry: String,
    timeout: Duration,
) -> Result<EppClient<C>, Error>
where
    C: Connector,
{
    EppClient::new(connector, registry, timeout).await
}

#[cfg(feature = "tokio-rustls")]
pub struct RustlsConnector {
    inner: TlsConnector,
    domain: ServerName,
    server: (String, u16),
}

impl RustlsConnector {
    pub async fn new(
        server: (String, u16),
        identity: Option<(Vec<Certificate>, PrivateKey)>,
    ) -> Result<Self, Error> {
        let mut roots = RootCertStore::empty();
        roots.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let builder = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots);

        let config = match identity {
            Some((certs, key)) => {
                let certs = certs
                    .into_iter()
                    .map(|cert| tokio_rustls::rustls::Certificate(cert.0))
                    .collect();
                builder
                    .with_single_cert(certs, tokio_rustls::rustls::PrivateKey(key.0))
                    .map_err(|e| Error::Other(e.into()))?
            }
            None => builder.with_no_client_auth(),
        };

        let domain = server.0.as_str().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid domain: {}", server.0),
            )
        })?;

        Ok(Self {
            inner: TlsConnector::from(Arc::new(config)),
            domain,
            server,
        })
    }
}

#[cfg(feature = "tokio-rustls")]
#[async_trait]
impl Connector for RustlsConnector {
    type Connection = TlsStream<TcpStream>;

    async fn connect(&self, timeout: Duration) -> Result<Self::Connection, Error> {
        info!("Connecting to server: {}:{}", self.server.0, self.server.1);
        let addr = match lookup_host(&self.server).await?.next() {
            Some(addr) => addr,
            None => {
                return Err(Error::Io(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid host: {}", &self.server.0),
                )))
            }
        };

        let stream = TcpStream::connect(addr).await?;
        let future = self.inner.connect(self.domain.clone(), stream);
        connection::timeout(timeout, future).await
    }
}

#[async_trait]
pub trait Connector {
    type Connection: AsyncRead + AsyncWrite + Unpin;

    async fn connect(&self, timeout: Duration) -> Result<Self::Connection, Error>;
}
