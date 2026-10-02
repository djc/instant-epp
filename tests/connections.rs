//! Regression tests for the connection leads from the security audit
//!
//! `log_to_stdout`, `len_bytes`, `xml` and `build_stream` are copies of the helpers in
//! `tests/basic.rs`.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;
use tokio::time::timeout;
use tokio_test::io::Builder;

use instant_epp::client::{Connector, EppClient};
use instant_epp::login::Login;
use instant_epp::Error;

/// The registry sets the frame length, so the client must cap the allocation.
#[tokio::test]
async fn frame_length_above_default_limit_is_rejected() {
    let _guard = log_to_stdout();

    let mut builder = Builder::new();
    builder.read(&[0xFF, 0xFF, 0xFF, 0xFF]);

    match connect(builder).await {
        Err(Error::ResponseTooLarge { expected, max }) => {
            assert_eq!(expected, u32::MAX as usize);
            assert_eq!(max, 1024 * 1024);
        }
        Err(err) => panic!("expected ResponseTooLarge, got {err:?}"),
        Ok(_) => panic!("expected ResponseTooLarge, got a client"),
    }
}

/// `set_max_read_buf` applies to the responses after the greeting.
#[tokio::test]
async fn frame_length_above_configured_limit_is_rejected() {
    let _guard = log_to_stdout();

    let mut builder = build_stream(&["response/greeting.xml", "request/login.xml"]);
    builder.read(&2052u32.to_be_bytes());

    let mut client = connect(builder).await.unwrap();
    client.set_max_read_buf(1024);

    match client.transact(&login(), CLTRID).await {
        Err(Error::ResponseTooLarge { expected, max }) => {
            assert_eq!(expected, 2052);
            assert_eq!(max, 1024);
        }
        Err(err) => panic!("expected ResponseTooLarge, got {err:?}"),
        Ok(rsp) => panic!("expected ResponseTooLarge, got {rsp:?}"),
    }
}

/// A length below the bytes in the first read used to shrink the buffer below the read
/// offset, and the next read panicked.
#[tokio::test]
async fn frame_length_below_bytes_already_read_is_rejected() {
    let _guard = log_to_stdout();

    // The header declares 6 bytes, but the first read already holds 8.
    let mut builder = Builder::new();
    builder.read(&[0, 0, 0, 6, b'a', b'b', b'c', b'd']);

    match connect(builder).await {
        Err(Error::Io(err)) => assert_eq!(err.kind(), io::ErrorKind::InvalidData),
        Err(err) => panic!("expected an InvalidData error, got {err:?}"),
        Ok(_) => panic!("expected an InvalidData error, got a client"),
    }
}

/// A length below the 4-byte header is invalid.
#[tokio::test]
async fn frame_length_below_header_size_is_rejected() {
    let _guard = log_to_stdout();

    let mut builder = Builder::new();
    builder.read(&[0, 0, 0, 0]);

    match connect(builder).await {
        Err(Error::Io(err)) => assert_eq!(err.kind(), io::ErrorKind::InvalidData),
        Err(err) => panic!("expected an InvalidData error, got {err:?}"),
        Ok(_) => panic!("expected an InvalidData error, got a client"),
    }
}

/// A frame that arrives in one read is complete. The client must not read again, and
/// the result must hold only the frame's body.
#[tokio::test]
async fn response_in_one_read() {
    let _guard = log_to_stdout();

    let greeting = "<greeting/>";
    let mut builder = Builder::new();
    builder.read(&frame(greeting));

    let client = connect(builder).await.unwrap();
    assert_eq!(client.xml_greeting(), greeting);

    let mut builder = Builder::new();
    builder.read(&frame("<greeting/>"));
    builder.write(&frame("<request/>"));
    builder.read(&frame("<response/>"));

    let mut client = connect(builder).await.unwrap();
    let rsp = timeout(Duration::from_secs(1), client.transact_xml("<request/>"))
        .await
        .expect("transact hung on a frame that was already complete")
        .unwrap();
    assert_eq!(rsp, "<response/>");
}

/// A complete one-read frame for a dropped request must be skipped, and the queued
/// request must then get its own response.
#[tokio::test]
async fn response_in_one_read_after_dropped_request() {
    let _guard = log_to_stdout();

    let mut builder = Builder::new();
    builder.read(&frame("<greeting/>"));
    builder.write(&frame("<first/>"));
    builder.wait(Duration::from_millis(100));
    builder.read(&frame("<first-response/>"));
    builder.write(&frame("<second/>"));
    builder.read(&frame("<second-response/>"));

    let mut client = connect(builder).await.unwrap();

    // Drop the first request while its response is pending.
    timeout(Duration::from_millis(10), client.transact_xml("<first/>"))
        .await
        .unwrap_err();

    let rsp = timeout(Duration::from_secs(1), client.transact_xml("<second/>"))
        .await
        .expect("transact hung on a frame that was already complete")
        .unwrap();
    assert_eq!(rsp, "<second-response/>");
}

/// A failed reconnect used to clear the in-flight request but keep the old stream, so
/// the next request got the response of the earlier request.
#[tokio::test]
async fn failed_reconnect_keeps_in_flight_request() {
    let _guard = log_to_stdout();

    // Each frame arrives as a header read and a body read.
    let mut builder = Builder::new();
    builder.read(&len_bytes("<greeting/>")).read(b"<greeting/>");
    builder.write(&frame("<first/>"));
    builder.wait(Duration::from_millis(100));
    builder
        .read(&len_bytes("<first-response/>"))
        .read(b"<first-response/>");
    builder.write(&frame("<second/>"));
    builder
        .read(&len_bytes("<second-response/>"))
        .read(b"<second-response/>");

    // The second connect fails.
    let connector = MockConnector::new(vec![Some(builder), None]);
    let mut client = EppClient::new(connector, "test".into(), Duration::from_secs(5))
        .await
        .unwrap();

    // Drop the first request while its response is pending.
    timeout(Duration::from_millis(10), client.transact_xml("<first/>"))
        .await
        .unwrap_err();

    assert!(client.reconnect().await.is_err());

    // Continuing here is not desired but could happen in complex code.
    // We should be correct instead of relying on the user to avoid this situation.
    // The old stream still holds the first response. The second request must skip it.
    let rsp = timeout(Duration::from_secs(1), client.transact_xml("<second/>"))
        .await
        .expect("transact hung")
        .unwrap();
    assert_eq!(rsp, "<second-response/>");
}

struct TestWriter;

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!(
            "{}",
            str::from_utf8(buf).expect("tried to log invalid UTF-8")
        );
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

fn log_to_stdout() -> tracing::subscriber::DefaultGuard {
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(|| TestWriter)
        .finish();
    tracing::subscriber::set_default(sub)
}

fn len_bytes(bytes: &str) -> [u8; 4] {
    ((bytes.len() as u32) + 4).to_be_bytes()
}

fn xml(path: &str) -> String {
    let ws_regex = Regex::new(r"[\s]{2,}").unwrap();
    let end_regex = Regex::new(r"\?>").unwrap();

    let mut f = File::open(format!("tests/resources/{path}")).unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();

    if !buf.is_empty() {
        let mat = end_regex.find(buf.as_str()).unwrap();
        let start = mat.end();
        buf = format!(
            "{}\r\n{}",
            &buf[..start],
            ws_regex.replace_all(&buf[start..], "")
        );
    }

    buf
}

fn build_stream(units: &[&str]) -> Builder {
    let mut builder = Builder::new();
    for (i, path) in units.iter().enumerate() {
        let buf = xml(path);
        match i % 2 {
            0 => builder.read(&len_bytes(&buf)).read(buf.as_bytes()),
            1 => builder.write(&len_bytes(&buf)).write(buf.as_bytes()),
            _ => unreachable!(),
        };
    }

    builder
}

/// Hands out one mock stream for each `connect` call; `None` makes that call fail
struct MockConnector {
    streams: Mutex<VecDeque<Option<Builder>>>,
}

impl MockConnector {
    fn new(streams: Vec<Option<Builder>>) -> Self {
        Self {
            streams: Mutex::new(streams.into()),
        }
    }
}

#[async_trait]
impl Connector for MockConnector {
    type Connection = tokio_test::io::Mock;

    async fn connect(&self, _: Duration) -> Result<Self::Connection, Error> {
        match self.streams.lock().unwrap().pop_front() {
            Some(Some(mut builder)) => Ok(builder.build()),
            _ => Err(Error::Io(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "mock connect failure",
            ))),
        }
    }
}

/// Connect over one mock stream. A hung read fails the test instead of blocking it.
async fn connect(builder: Builder) -> Result<EppClient<MockConnector>, Error> {
    let connector = MockConnector::new(vec![Some(builder)]);
    timeout(
        Duration::from_secs(1),
        EppClient::new(connector, "test".into(), Duration::from_secs(5)),
    )
    .await
    .expect("connect hung while reading the greeting")
}

/// A complete RFC 5734 frame: the 4-byte header followed by `body`
fn frame(body: &str) -> Vec<u8> {
    [&len_bytes(body)[..], body.as_bytes()].concat()
}

fn login() -> Login<'static> {
    Login::new(
        "username",
        "password",
        Some("new-password"),
        Some(&["http://schema.ispapi.net/epp/xml/keyvalue-1.0"]),
    )
}

const CLTRID: &str = "cltrid:1626454866";
