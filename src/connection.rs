//! Manages registry connections and reading/writing to them

use std::future::Future;
use std::time::Duration;
use std::{io, str, u32};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info};

use crate::connect::Connector;
use crate::error::Error;

/// EPP Connection struct with some metadata for the connection
pub(crate) struct EppConnection<C: Connector> {
    pub registry: String,
    connector: C,
    stream: C::Connection,
    pub greeting: String,
    timeout: Duration,
    state: ConnectionState,
}

impl<C: Connector> EppConnection<C> {
    pub(crate) async fn new(
        connector: C,
        registry: String,
        timeout: Duration,
    ) -> Result<Self, Error> {
        let mut this = Self {
            registry,
            stream: connector.connect(timeout).await?,
            connector,
            greeting: String::new(),
            timeout,
            state: Default::default(),
        };

        this.greeting = this.read_epp_response().await?;
        this.state = ConnectionState::Open;
        Ok(this)
    }

    /// Constructs an EPP XML request in the required form and sends it to the server
    async fn write_epp_request(&mut self, content: &str) -> Result<(), Error> {
        let len = content.len();

        let buf_size = len + 4;
        let mut buf: Vec<u8> = vec![0u8; buf_size];

        let len = len + 4;
        let len_u32: [u8; 4] = u32::to_be_bytes(len.try_into()?);

        buf[..4].clone_from_slice(&len_u32);
        buf[4..].clone_from_slice(content.as_bytes());

        let wrote = timeout(self.timeout, self.stream.write(&buf)).await?;
        debug!(registry = self.registry, "Wrote {} bytes", wrote);
        Ok(())
    }

    /// Receives response from the socket and converts it into an EPP XML string
    async fn read_epp_response(&mut self) -> Result<String, Error> {
        let mut buf = [0u8; 4];
        timeout(self.timeout, self.stream.read_exact(&mut buf)).await?;

        let buf_size: usize = u32::from_be_bytes(buf).try_into()?;

        let message_size = buf_size - 4;
        debug!(
            registry = self.registry,
            "Response buffer size: {}", message_size
        );

        let mut buf = vec![0; message_size];
        let mut read_size: usize = 0;

        loop {
            let read = timeout(self.timeout, self.stream.read(&mut buf[read_size..])).await?;
            debug!(registry = self.registry, "Read: {} bytes", read);

            read_size += read;
            debug!(registry = self.registry, "Total read: {} bytes", read_size);

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

    pub(crate) async fn reconnect(&mut self) -> Result<(), Error> {
        debug!(registry = self.registry, "reconnecting");
        self.state = ConnectionState::Opening;
        self.stream = self.connector.connect(self.timeout).await?;
        self.greeting = self.read_epp_response().await?;
        self.state = ConnectionState::Open;
        Ok(())
    }

    /// Sends an EPP XML request to the registry and return the response
    /// receieved to the request
    pub(crate) async fn transact(&mut self, content: &str) -> Result<String, Error> {
        if self.state != ConnectionState::Open {
            debug!(registry = self.registry, " connection not ready");
            self.reconnect().await?;
        }

        debug!(registry = self.registry, " request: {}", content);
        self.write_epp_request(content).await?;

        let response = self.read_epp_response().await?;
        debug!(registry = self.registry, " response: {}", response);

        Ok(response)
    }

    /// Closes the socket and shuts the connection
    pub(crate) async fn shutdown(&mut self) -> Result<(), Error> {
        info!(registry = self.registry, "Closing connection");
        self.state = ConnectionState::Closing;
        timeout(self.timeout, self.stream.shutdown()).await?;
        self.state = ConnectionState::Closed;
        Ok(())
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
