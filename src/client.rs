use std::time::Duration;

#[cfg(feature = "__rustls")]
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tracing::{debug, error};

use crate::common::NoExtension;
pub use crate::connection::Connector;
use crate::connection::EppConnection;
use crate::error::Error;
use crate::hello::{Greeting, Hello};
use crate::request::{Command, CommandWrapper, Extension, Transaction};
use crate::response::{Response, ResponseStatus};
use crate::xml;

/// An `EppClient` provides an interface to sending EPP requests to a registry
///
/// Once initialized, the EppClient instance can serialize EPP requests to XML and send them
/// to the registry and deserialize the XML responses from the registry to local types.
///
/// # Examples
///
/// ```no_run
/// # use std::collections::HashMap;
/// # use std::net::ToSocketAddrs;
/// # use std::time::Duration;
/// #
/// use instant_epp::EppClient;
/// use instant_epp::domain::DomainCheck;
/// use instant_epp::common::NoExtension;
///
/// # #[cfg(feature = "rustls")]
/// # #[tokio::main]
/// # async fn main() {
/// // Create an instance of EppClient
/// let timeout = Duration::from_secs(5);
/// let mut client = match EppClient::connect("registry_name".to_string(), ("example.com".to_owned(), 7000), None, timeout).await {
///     Ok(client) => client,
///     Err(e) => panic!("Failed to create EppClient: {}",  e)
/// };
///
/// // Make a EPP Hello call to the registry
/// let greeting = client.hello().await.unwrap();
/// println!("{:?}", greeting);
///
/// // Execute an EPP Command against the registry with distinct request and response objects
/// let domain_check = DomainCheck { domains: &["eppdev.com", "eppdev.net"] };
/// let response = client.transact(&domain_check, "transaction-id").await.unwrap();
/// response
///     .res_data()
///     .unwrap()
///     .list
///     .iter()
///     .for_each(|chk| println!("Domain: {}, Available: {}", chk.inner.id, chk.inner.available));
/// # }
/// #
/// # #[cfg(not(feature = "rustls"))]
/// # fn main() {}
/// ```
///
/// The output would look like this:
///
/// ```text
/// Domain: eppdev.com, Available: 1
/// Domain: eppdev.net, Available: 1
/// ```
pub struct EppClient<C: Connector> {
    connection: EppConnection<C>,
}

#[cfg(feature = "__rustls")]
impl EppClient<RustlsConnector> {
    /// Connect to the specified `addr` and `hostname` over TLS
    ///
    /// The `registry` is used as a name in internal logging; `host` provides the host name
    /// and port to connect to), `hostname` is sent as the TLS server name indication and
    /// `identity` provides optional TLS client authentication (using) rustls as the TLS
    /// implementation. The `timeout` limits the time spent on any underlying network operations.
    ///
    /// Alternatively, use `EppClient::new()` with any established `AsyncRead + AsyncWrite + Unpin`
    /// implementation.
    pub async fn connect(
        registry: String,
        server: (String, u16),
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        timeout: Duration,
    ) -> Result<Self, Error> {
        let builder =
            RustlsConnector::builder(server).map_err(|err| Error::Other(Box::new(err)))?;
        let builder = match identity {
            Some((certs, key)) => builder.client_auth(certs, key),
            None => builder,
        };

        let connector = builder.build().map_err(|err| Error::Other(Box::new(err)))?;
        Self::new(connector, registry, timeout).await
    }
}

impl<C: Connector> EppClient<C> {
    /// Create an `EppClient` from an already established connection
    pub async fn new(connector: C, registry: String, timeout: Duration) -> Result<Self, Error> {
        Ok(Self {
            connection: EppConnection::new(connector, registry, timeout).await?,
        })
    }

    /// Executes an EPP Hello call and returns the response as a `Greeting`
    pub async fn hello(&mut self) -> Result<Greeting, Error> {
        let xml = xml::serialize(Hello)?;

        debug!("{}: hello: {}", self.connection.registry, &xml);
        let response = self.connection.transact(&xml)?.await?;
        debug!("{}: greeting: {}", self.connection.registry, &response);

        xml::deserialize::<Greeting>(&response)
    }

    pub async fn transact<'c, 'e, Cmd, Ext>(
        &mut self,
        data: impl Into<RequestData<'c, 'e, Cmd, Ext>>,
        id: &str,
    ) -> Result<Response<Cmd::Response, Ext::Response>, Error>
    where
        Cmd: Transaction<Ext> + Command + 'c,
        Ext: Extension + 'e,
    {
        let data = data.into();
        let document = CommandWrapper::new(data.command, data.extension, id);
        let xml = xml::serialize(&document)?;

        debug!("{}: request: {}", self.connection.registry, &xml);
        let response = self.connection.transact(&xml)?.await?;
        debug!("{}: response: {}", self.connection.registry, &response);

        let rsp = match xml::deserialize::<Response<Cmd::Response, Ext::Response>>(&response) {
            Ok(rsp) => rsp,
            Err(e) => {
                error!(%response, "failed to deserialize response for transaction: {e}");
                return Err(e);
            }
        };

        if rsp.result.code.is_success() {
            return Ok(rsp);
        }

        let err = crate::error::Error::Command(Box::new(ResponseStatus {
            result: rsp.result,
            tr_ids: rsp.tr_ids,
        }));

        Err(err)
    }

    /// Accepts raw EPP XML and returns the raw EPP XML response to it.
    /// Not recommended for direct use but sometimes can be useful for debugging
    pub async fn transact_xml(&mut self, xml: &str) -> Result<String, Error> {
        self.connection.transact(xml)?.await
    }

    /// Returns the greeting received on establishment of the connection in raw xml form
    pub fn xml_greeting(&self) -> String {
        String::from(&self.connection.greeting)
    }

    /// Returns the greeting received on establishment of the connection as an `Greeting`
    pub fn greeting(&self) -> Result<Greeting, Error> {
        xml::deserialize::<Greeting>(&self.connection.greeting)
    }

    pub async fn reconnect(&mut self) -> Result<(), Error> {
        self.connection.reconnect().await
    }

    pub async fn shutdown(mut self) -> Result<(), Error> {
        self.connection.shutdown().await
    }
}

#[derive(Debug)]
pub struct RequestData<'c, 'e, C, E> {
    pub(crate) command: &'c C,
    pub(crate) extension: Option<&'e E>,
}

impl<'c, C: Command> From<&'c C> for RequestData<'c, 'static, C, NoExtension> {
    fn from(command: &'c C) -> Self {
        Self {
            command,
            extension: None,
        }
    }
}

impl<'c, 'e, C: Command, E: Extension> From<(&'c C, &'e E)> for RequestData<'c, 'e, C, E> {
    fn from((command, extension): (&'c C, &'e E)) -> Self {
        Self {
            command,
            extension: Some(extension),
        }
    }
}

// Manual impl because this does not depend on whether `C` and `E` are `Clone`
impl<C, E> Clone for RequestData<'_, '_, C, E> {
    fn clone(&self) -> Self {
        *self
    }
}

// Manual impl because this does not depend on whether `C` and `E` are `Copy`
impl<C, E> Copy for RequestData<'_, '_, C, E> {}

#[cfg(feature = "__rustls")]
pub use rustls_connector::RustlsConnector;

#[cfg(feature = "__rustls")]
mod rustls_connector {
    use std::io;
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use rustls_platform_verifier::BuilderVerifierExt;
    use tokio::net::lookup_host;
    use tokio::net::TcpStream;
    use tokio_rustls::client::TlsStream;
    use tokio_rustls::rustls::pki_types::InvalidDnsNameError;
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
    use tokio_rustls::rustls::ClientConfig;
    use tokio_rustls::TlsConnector;
    use tracing::info;

    use crate::connection::{self, Connector};
    use crate::error::Error;

    pub struct RustlsConnector {
        inner: TlsConnector,
        server_name: ServerName<'static>,
        server: (String, u16),
    }

    impl RustlsConnector {
        /// Create a builder with the given `server` (consisting of a hostname and port)
        pub fn builder(
            server: (String, u16),
        ) -> Result<RustlsConnectorBuilder, InvalidDnsNameError> {
            Ok(RustlsConnectorBuilder {
                server_name: ServerName::try_from(server.0.as_str())?.to_owned(),
                server,
                identity: None,
            })
        }
    }

    #[async_trait]
    impl Connector for RustlsConnector {
        type Connection = TlsStream<TcpStream>;

        async fn connect(&self, timeout: Duration) -> Result<Self::Connection, Error> {
            info!("connecting to server: {}:{}", self.server.0, self.server.1);
            let addr = match lookup_host(&self.server).await?.next() {
                Some(addr) => addr,
                None => {
                    return Err(Error::Io(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid host: {}", &self.server.0),
                    )))
                }
            };

            let stream = TcpStream::connect(addr).await?;
            let future = self.inner.connect(self.server_name.clone(), stream);
            connection::timeout(timeout, future).await
        }
    }

    pub struct RustlsConnectorBuilder {
        server: (String, u16),
        server_name: ServerName<'static>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    }

    impl RustlsConnectorBuilder {
        /// Enable client authentication
        ///
        /// Only used when `build()` is called.
        pub fn client_auth(
            mut self,
            certs: Vec<CertificateDer<'static>>,
            key: PrivateKeyDer<'static>,
        ) -> Self {
            self.identity = Some((certs, key));
            self
        }

        /// Use the given `config` for the TLS connector
        ///
        /// Any client authentication set with `client_auth` will be ignored.
        pub fn build_with_config(self, config: Arc<ClientConfig>) -> RustlsConnector {
            let Self {
                server,
                server_name,
                identity: _identity,
            } = self;

            RustlsConnector {
                inner: TlsConnector::from(config),
                server_name,
                server,
            }
        }

        /// Build a new [`ClientConfig`]
        pub fn build(self) -> Result<RustlsConnector, tokio_rustls::rustls::Error> {
            let Self {
                server,
                server_name,
                identity,
            } = self;

            let builder = ClientConfig::builder().with_platform_verifier()?;
            let config = match identity {
                Some((certs, key)) => builder.with_client_auth_cert(certs, key)?,
                None => builder.with_no_client_auth(),
            };

            Ok(RustlsConnector {
                inner: TlsConnector::from(Arc::new(config)),
                server_name,
                server,
            })
        }
    }
}
