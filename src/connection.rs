//! Manages registry connections and reading/writing to them

use std::borrow::Cow;
use std::convert::TryInto;
use std::future::Future;
use std::time::Duration;
use std::{io, str, u32};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::connect::Connector;
use crate::error::Error;
use crate::hello::HelloDocument;
use crate::xml;

/// EPP Connection
///
/// This is the I/O half, returned when creating a new connection, that performs the actual I/O and thus
/// should be spawned in it's own task.
///
/// [`EppConnection`] provides a [`EppConnection::run`](EppConnection::run) method, which only resolves when the connection is closed,
/// either because a fatal error has occurred, or because its associated [`EppClient`](super::EppClient) has been dropped
/// and all outstanding work has been completed.
///
/// # Keepalive (Idle Timeout)
///
/// EppConnection supports a keepalive mechanism.
/// When `idle_timeout` is set, every time the timeout reaches zero while waiting for a new request from the
/// [`EppClient`](super::EppClient), a `<hello>` request is sent to the epp server.
/// This is in line with VeriSign's guidelines. VeriSign uses an idle timeout of 10 minutes and an absolute timeout of 24h.
/// Choosing an `idle_timeout` of 8 minutes should be sufficient to not run into VeriSign's idle timeout.
/// Other registry operators might need other values.
///
/// # Reconnect (Absolute Timeout)
///
/// Reconnecting, to gracefully allow a [`EppConnection`] to be "active", is currently not implemented. But a reconnect
/// command is present to initiate the reconnect from the outside
pub struct EppConnection<C: Connector> {
    registry: Cow<'static, str>,
    connector: C,
    stream: C::Connection,
    greeting: String,
    timeout: Duration,
    idle_timeout: Option<Duration>,
    /// A receiver for receiving requests from [`EppClients`](super::client::EppClient) for the underlying connection.
    receiver: mpsc::UnboundedReceiver<Request>,
    state: ConnectionState,
}

impl<C: Connector> EppConnection<C> {
    pub(crate) async fn new(
        connector: C,
        registry: Cow<'static, str>,
        receiver: mpsc::UnboundedReceiver<Request>,
        request_timeout: Duration,
        idle_timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        let mut this = Self {
            registry,
            stream: connector.connect(request_timeout).await?,
            connector,
            receiver,
            greeting: String::new(),
            timeout: request_timeout,
            idle_timeout,
            state: Default::default(),
        };

        this.greeting = this.read_epp_response().await?;
        this.state = ConnectionState::Open;
        Ok(this)
    }

    /// Runs the connection
    ///
    /// This will loops and awaits new requests from the client half and sends the request to the epp server
    /// awaiting a response.
    ///
    /// Spawn this in a task and await run to resolve.
    /// This resolves when the connection to the epp server gets dropped.
    ///
    /// # Examples
    /// ```[no_compile]
    /// let mut connection = <obtained via connect::connect()>
    /// tokio::spawn(async move {
    ///     if let Err(err) = connection.run().await {
    ///         error!("connection failed: {err}")
    ///     }
    /// });
    pub async fn run(&mut self) -> Result<(), Error> {
        while let Some(message) = self.message().await {
            match message {
                Ok(message) => info!("{message}"),
                Err(err) => {
                    error!("{err}");
                    break;
                }
            }
        }
        trace!("stopping EppConnection task");
        Ok(())
    }

    /// Sends the given content to the used [`Connector::Connection`]
    ///
    /// Returns an EOF error when writing to the stream results in 0 bytes written.
    async fn write_epp_request(&mut self, content: &str) -> Result<(), Error> {
        let len = content.len();

        let buf_size = len + 4;
        let mut buf: Vec<u8> = vec![0u8; buf_size];

        let len = len + 4;
        let len_u32: [u8; 4] = u32::to_be_bytes(len.try_into()?);

        buf[..4].clone_from_slice(&len_u32);
        buf[4..].clone_from_slice(content.as_bytes());

        let wrote = timeout(self.timeout, self.stream.write(&buf)).await?;
        // A write return value of 0 means the underlying socket
        // does no longer accept any data.
        if wrote == 0 {
            warn!("Got EOF while writing");
            self.state = ConnectionState::Closed;
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("{}: unexpected eof", self.registry),
            )
            .into());
        }

        debug!(registry = %self.registry, "Wrote {} bytes", wrote);
        Ok(())
    }

    /// Receives response from the socket and converts it into an EPP XML string
    async fn read_epp_response(&mut self) -> Result<String, Error> {
        // We're looking for the frame header which tells us how long the response will be.
        // The frame header is a 32-bit (4-byte) big-endian unsigned integer.
        let mut buf = [0u8; 4];
        timeout(self.timeout, self.stream.read_exact(&mut buf)).await?;

        let buf_size: usize = u32::from_be_bytes(buf).try_into()?;

        let message_size = buf_size - 4;
        debug!(
            registry = %self.registry,
            "Response buffer size: {}", message_size
        );

        let mut buf = vec![0; message_size];
        let mut read_size: usize = 0;

        loop {
            let read = timeout(self.timeout, self.stream.read(&mut buf[read_size..])).await?;
            debug!(registry = %self.registry, "Read: {} bytes", read);

            read_size += read;
            debug!(registry = %self.registry, "Total read: {} bytes", read_size);

            if read == 0 {
                self.state = ConnectionState::Closed;
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("{}: unexpected eof", self.registry),
                )
                .into());
            } else if read_size >= message_size {
                break;
            }
        }

        Ok(String::from_utf8(buf)?)
    }

    async fn reconnect(&mut self) -> Result<(), Error> {
        debug!(registry = %self.registry, "reconnecting");
        self.state = ConnectionState::Opening;
        self.stream = self.connector.connect(self.timeout).await?;
        self.greeting = self.read_epp_response().await?;
        self.state = ConnectionState::Open;
        Ok(())
    }

    async fn wait_for_shutdown(&mut self) -> Result<(), io::Error> {
        self.state = ConnectionState::Closing;
        match self.stream.shutdown().await {
            Ok(_) => {
                self.state = ConnectionState::Closed;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn request_or_keepalive(&mut self) -> Result<Option<Request>, Error> {
        loop {
            let Some(idle_timeout) = self.idle_timeout else {
                // We do not have any keep alive set, just forward to waiting for a request.
                return Ok(self.receiver.recv().await);
            };
            trace!(registry = %self.registry, "Waiting for {idle_timeout:?} for new request until keepalive");
            match tokio::time::timeout(idle_timeout, self.receiver.recv()).await {
                Ok(request) => return Ok(request),
                Err(_) => {
                    self.keepalive().await?;
                    // We sent the keepalive. Go back to wait for requests.
                    continue;
                }
            }
        }
    }

    async fn keepalive(&mut self) -> Result<(), Error> {
        trace!(registry = %self.registry, "Sending keepalive hello");
        // Send hello
        let request = xml::serialize(&HelloDocument::default())?;
        self.write_epp_request(&request).await?;

        // Await new greeting
        self.greeting = self.read_epp_response().await?;
        Ok(())
    }

    /// This is the main method of the I/O tasks
    ///
    /// It will try to get a request, write it to the wire and waits for the response.
    ///
    /// Once this returns `None`, or `Ok(Err(_))`, the connection is expected to be closed.
    async fn message(&mut self) -> Option<Result<Cow<'static, str>, Error>> {
        // In theory this can be even speed up as the underlying stream is in our case bi-directional.
        // But as the EPP RFC does not guarantee the order of responses we would need to
        // match based on the transactions id. We can look into adding support for this in
        // future.
        loop {
            if self.state == ConnectionState::Closed {
                return None;
            }

            // Wait for new request or send a keepalive
            let request = match self.request_or_keepalive().await {
                Ok(request) => request,
                Err(err) => return Some(Err(err)),
            };
            let Some(request) = request  else {
                // The client got dropped. We can close the connection.
                match self.wait_for_shutdown().await {
                    Ok(_) => return None,
                    Err(err) => return Some(Err(err.into())),
                }
            };

            let response = match request.request {
                RequestMessage::Greeting => Ok(self.greeting.clone()),
                RequestMessage::Request(request) => {
                    if let Err(err) = self.write_epp_request(&request).await {
                        return Some(Err(err));
                    }
                    timeout(self.timeout, self.read_epp_response()).await
                }
                RequestMessage::Reconnect => match self.reconnect().await {
                    Ok(_) => Ok(self.greeting.clone()),
                    Err(err) => {
                        // In this case we are not sure if the connection is in tact. Best we error out.
                        let _ = request.sender.send(Err(Error::Reconnect)).await;
                        return Some(Err(err));
                    }
                },
            };

            // Awaiting `send` should not block this I/O tasks unless we try to write multiple responses to the same bounded channel.
            // As this crate is structured to create a new bounded channel for each request, this ok here.
            if request.sender.send(response).await.is_err() {
                // If the receive half of the sender is dropped, (i.e. the `Client`s `Future` is canceled)
                // we can just ignore the err here and return to let `run` print something for this task.
                return Some(Ok("request was canceled. Client dropped.".into()));
            }
        }
    }
}

pub(crate) async fn timeout<T, E: Into<Error>>(
    timeout: Duration,
    fut: impl Future<Output = Result<T, E>>,
) -> Result<T, Error> {
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(Error::Timeout),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum ConnectionState {
    #[default]
    Opening,
    Open,
    Closing,
    Closed,
}

pub(crate) struct Request {
    pub(crate) request: RequestMessage,
    pub(crate) sender: mpsc::Sender<Result<String, Error>>,
}

pub(crate) enum RequestMessage {
    /// Request the stored server greeting
    Greeting,
    /// Reconnect the underlying [`Connector::Connection`]
    Reconnect,
    /// Raw request to be sent to the connected EPP Server
    Request(String),
}
