use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::mpsc;
use tracing::{debug, error};

use crate::common::NoExtension;
pub use crate::connect::Connector;
use crate::connection::{Request, RequestMessage};
use crate::error::Error;
use crate::hello::{Greeting, GreetingDocument, HelloDocument};
use crate::request::{Command, CommandDocument, Extension, Transaction};
use crate::response::{Response, ResponseDocument, ResponseStatus};
use crate::xml;

/// EPP Client
///
/// Provides an interface to send EPP requests to a registry
///
/// Once initialized, the [`EppClient`] instance is the API half that is returned when creating a new connection.
/// It can serialize EPP requests to XML and send them to the registry and deserialize the XML responses from the
/// registry to local types.
///
/// # Examples
///
/// ```no_run
/// # use std::collections::HashMap;
/// # use std::net::ToSocketAddrs;
/// # use std::time::Duration;
/// #
/// use epp_client::connect::connect;
/// use epp_client::domain::DomainCheck;
/// use epp_client::common::NoExtension;
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create an instance of EppClient
/// let timeout = Duration::from_secs(5);
/// let (mut client, mut connection) = match connect("registry_name".into(), ("example.com".into(), 7000), None, timeout, None).await {
///     Ok(client) => client,
///     Err(e) => panic!("Failed to create EppClient: {}",  e)
/// };
///
/// tokio::spawn(async move {
///     connection.run().await.unwrap();
/// });
///
/// // Make a EPP Hello call to the registry
/// let greeting = client.hello().await.unwrap();
/// println!("{:?}", greeting);
///
/// // Execute an EPP Command against the registry with distinct request and response objects
/// let domain_check = DomainCheck { domains: &["eppdev.com", "eppdev.net"] };
/// let response = client.transact(&domain_check, "transaction-id").await.unwrap();
/// response.res_data.unwrap().list
///     .iter()
///     .for_each(|chk| println!("Domain: {}, Available: {}", chk.id, chk.available));
/// # }
/// ```
///
/// The output would look like this:
///
/// ```text
/// Domain: eppdev.com, Available: 1
/// Domain: eppdev.net, Available: 1
/// ```
pub struct EppClient {
    inner: Arc<InnerClient>,
}

impl EppClient {
    pub(crate) fn new(sender: mpsc::UnboundedSender<Request>, registry: Cow<'static, str>) -> Self {
        Self {
            inner: Arc::new(InnerClient { sender, registry }),
        }
    }

    /// Executes an EPP Hello call and returns the response as a `Greeting`
    pub async fn hello(&mut self) -> Result<Greeting, Error> {
        let xml = xml::serialize(&HelloDocument::default())?;

        debug!(registry = %self.inner.registry, "hello: {}", &xml);
        let response = self.inner.send(xml)?.await?;
        debug!(
            registry = %self.inner.registry,
            "greeting: {}", &response
        );

        Ok(xml::deserialize::<GreetingDocument>(&response)?.data)
    }

    /// Sends a EPP request and await its response
    ///
    /// The given transactions id is not checked internally when build in release mode.
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
        let document = CommandDocument::new(data.command, data.extension, id);
        let xml = xml::serialize(&document)?;

        debug!(registry = %self.inner.registry, "request: {}", &xml);
        let response = self.inner.send(xml)?.await?;
        debug!(
            registry = %self.inner.registry,
            "response: {}", &response
        );

        let rsp = xml::deserialize::<ResponseDocument<Cmd::Response, Ext::Response>>(&response)?;
        debug_assert!(rsp.data.tr_ids.client_tr_id.as_deref() == Some(id));

        if rsp.data.result.code.is_success() {
            return Ok(rsp.data);
        }

        let err = crate::error::Error::Command(Box::new(ResponseStatus {
            result: rsp.data.result,
            tr_ids: rsp.data.tr_ids,
        }));

        error!(
            registry = %self.inner.registry,
            %response,
            "Failed to deserialize response for transaction: {}", err
        );
        Err(err)
    }

    /// Accepts raw EPP XML and returns the raw EPP XML response to it.
    /// Not recommended for direct use but sometimes can be useful for debugging
    pub async fn transact_xml(&mut self, xml: String) -> Result<String, Error> {
        self.inner.send(xml)?.await
    }

    /// Returns the greeting received on establishment of the connection in raw xml form
    pub async fn xml_greeting(&self) -> Result<String, Error> {
        self.inner.xml_greeting().await
    }

    /// Returns the greeting received on establishment of the connection as an `Greeting`
    pub async fn greeting(&self) -> Result<Greeting, Error> {
        let greeting = self.inner.xml_greeting().await?;
        xml::deserialize::<GreetingDocument>(&greeting).map(|obj| obj.data)
    }

    /// Reconnects the underlying [`Connector::Connection`]
    pub async fn reconnect(&self) -> Result<Greeting, Error> {
        let greeting = self.inner.reconnect().await?;
        xml::deserialize::<GreetingDocument>(&greeting).map(|obj| obj.data)
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
impl<'c, 'e, C, E> Clone for RequestData<'c, 'e, C, E> {
    fn clone(&self) -> Self {
        Self {
            command: self.command,
            extension: self.extension,
        }
    }
}

// Manual impl because this does not depend on whether `C` and `E` are `Copy`
impl<'c, 'e, C, E> Copy for RequestData<'c, 'e, C, E> {}

struct InnerClient {
    sender: mpsc::UnboundedSender<Request>,
    pub registry: Cow<'static, str>,
}

impl InnerClient {
    fn send(&self, request: String) -> Result<InnerResponse, Error> {
        let (sender, receiver) = mpsc::channel(1);
        let request = Request {
            request: RequestMessage::Request(request),
            sender,
        };
        self.sender.send(request).map_err(|_| Error::Closed)?;

        Ok(InnerResponse { receiver })
    }

    /// Returns the greeting received on establishment of the connection in raw xml form
    async fn xml_greeting(&self) -> Result<String, Error> {
        let (sender, receiver) = mpsc::channel(1);
        let request = Request {
            request: RequestMessage::Greeting,
            sender,
        };
        self.sender.send(request).map_err(|_| Error::Closed)?;

        InnerResponse { receiver }.await
    }

    async fn reconnect(&self) -> Result<String, Error> {
        let (sender, receiver) = mpsc::channel(1);
        let request = Request {
            request: RequestMessage::Reconnect,
            sender,
        };
        self.sender.send(request).map_err(|_| Error::Closed)?;

        InnerResponse { receiver }.await
    }
}

// We do not need to parse any output at this point (we could do that),
// but for now we just store the receiver here.
pub(crate) struct InnerResponse {
    receiver: mpsc::Receiver<Result<String, Error>>,
}

impl Future for InnerResponse {
    type Output = Result<String, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.receiver.poll_recv(cx) {
            Poll::Ready(Some(response)) => Poll::Ready(response),
            Poll::Ready(None) => Poll::Ready(Err(Error::Closed)),
            Poll::Pending => Poll::Pending,
        }
    }
}
