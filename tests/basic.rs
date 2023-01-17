use std::fs::File;
use std::io::{self, Read, Write};
use std::str;
use std::time::Duration;

use async_trait::async_trait;
use epp_client::connect::connect_with_connector;
use regex::Regex;
use tokio::time::timeout;
use tokio_test::io::Builder;
use tracing::trace;

use epp_client::domain::{DomainCheck, DomainContact, DomainCreate, Period};
use epp_client::login::Login;
use epp_client::response::ResultCode;

const CLTRID: &str = "cltrid:1626454866";

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

    let mut f = File::open(format!("tests/resources/{}", path)).unwrap();
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

#[tokio::test]
async fn client() {
    let _guard = log_to_stdout();

    struct FakeConnector;

    #[async_trait]
    impl epp_client::client::Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, epp_client::Error> {
            Ok(build_stream(&[
                "response/greeting.xml",
                "request/login.xml",
                "response/login.xml",
                "request/domain/check.xml",
                "response/domain/check.xml",
            ])
            .build())
        }
    }

    let (mut client, mut connection) =
        connect_with_connector(FakeConnector, "test".into(), Duration::from_secs(5), None)
            .await
            .unwrap();

    tokio::spawn(async move {
        connection.run().await.unwrap();
    });

    assert_eq!(
        client.xml_greeting().await.unwrap(),
        xml("response/greeting.xml")
    );
    let rsp = client
        .transact(
            &Login::new(
                "username",
                "password",
                Some("new-password"),
                Some(&["http://schema.ispapi.net/epp/xml/keyvalue-1.0"]),
            ),
            CLTRID,
        )
        .await
        .unwrap();
    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);

    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);

    let rsp = client
        .transact(
            &DomainCheck {
                domains: &["eppdev.com", "eppdev.net"],
            },
            CLTRID,
        )
        .await
        .unwrap();
    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);

    let result = rsp.res_data().unwrap();
    assert_eq!(result.list[0].id, "eppdev.com");
}

#[tokio::test]
async fn dropped() {
    let _guard = log_to_stdout();

    struct FakeConnector;

    #[async_trait]
    impl epp_client::client::Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, epp_client::Error> {
            let mut builder = Builder::new();

            let buf = xml("response/greeting.xml");
            builder.read(&len_bytes(&buf)).read(buf.as_bytes());

            let buf = xml("request/login.xml");
            builder.write(&len_bytes(&buf)).write(buf.as_bytes());

            let buf = xml("response/login.xml");
            builder.read(&len_bytes(&buf)).read(buf.as_bytes());

            let buf = xml("request/domain/check.xml");
            builder.write(&len_bytes(&buf)).write(buf.as_bytes());

            // We add a wait here. We're going to timeout below as a way of dropping the future.
            builder.wait(Duration::from_millis(100));

            let buf = xml("response/domain/check.xml");
            builder.read(&len_bytes(&buf)).read(buf.as_bytes());

            let buf = xml("request/domain/create.xml");
            builder.write(&len_bytes(&buf)).write(buf.as_bytes());

            let buf = xml("response/domain/create.xml");
            builder.read(&len_bytes(&buf)).read(buf.as_bytes());

            Ok(builder.build())
        }
    }

    let (mut client, mut connection) =
        connect_with_connector(FakeConnector, "test".into(), Duration::from_secs(5), None)
            .await
            .unwrap();
    tokio::spawn(async move {
        connection.run().await.unwrap();
        trace!("connection future resolved successfully")
    });

    trace!("Trying to get greeting");
    assert_eq!(
        client.xml_greeting().await.unwrap(),
        xml("response/greeting.xml")
    );
    let rsp = client
        .transact(
            &Login::new(
                "username",
                "password",
                Some("new-password"),
                Some(&["http://schema.ispapi.net/epp/xml/keyvalue-1.0"]),
            ),
            CLTRID,
        )
        .await
        .unwrap();

    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);

    // Here, we add a 10ms timeout on the entire transaction. The mock stream
    // specifies that the caller will have to wait for 100ms after sending
    // the request before the response is returned. When `timeout()` returns
    // `Err(Elapsed)`, the `RequestFuture` inside the `Timeout` future is dropped,
    // leaving a half-finished request in the `EppConnection`.
    timeout(
        Duration::from_millis(10),
        client.transact(
            &DomainCheck {
                domains: &["eppdev.com", "eppdev.net"],
            },
            CLTRID,
        ),
    )
    .await
    .unwrap_err();

    let contacts = &[
        DomainContact {
            contact_type: "admin".into(),
            id: "eppdev-contact-3".into(),
        },
        DomainContact {
            contact_type: "tech".into(),
            id: "eppdev-contact-3".into(),
        },
        DomainContact {
            contact_type: "billing".into(),
            id: "eppdev-contact-3".into(),
        },
    ];

    // Then, we start another request (of a different type). This should push through the
    // remainder of the in-flight request before starting the new one, and succeed.
    let create = DomainCreate::new(
        "eppdev-1.com",
        Period::years(1).unwrap(),
        None,
        Some("eppdev-contact-3"),
        "epP4uthd#v",
        Some(contacts),
    );

    let rsp = client.transact(&create, CLTRID).await.unwrap();
    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);

    drop(client);
}

#[tokio::test]
async fn drop_client() {
    let _guard = log_to_stdout();

    struct FakeConnector;

    #[async_trait]
    impl epp_client::client::Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, epp_client::Error> {
            Ok(build_stream(&["response/greeting.xml"]).build())
        }
    }

    let (client, mut connection) =
        connect_with_connector(FakeConnector, "test".into(), Duration::from_secs(5), None)
            .await
            .unwrap();

    let handle = tokio::spawn(async move {
        connection.run().await.unwrap();
        trace!("connection future resolved successfully")
    });

    assert_eq!(
        client.xml_greeting().await.unwrap(),
        xml("response/greeting.xml")
    );

    drop(client);
    assert!(handle.await.is_ok());
}

#[tokio::test]
async fn keepalive() {
    let _guard = log_to_stdout();

    struct FakeConnector;

    #[async_trait]
    impl epp_client::client::Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, epp_client::Error> {
            Ok(build_stream(&[
                "response/greeting.xml",
                "request/login.xml",
                "response/login.xml",
                // The keepalive should kick in.
                // We set the keepalive to 100ms and wait 250ms which should yield two hello requests
                "request/hello.xml",
                "response/greeting.xml",
                "request/hello.xml",
                "response/greeting.xml",
                "request/domain/create.xml",
                "response/domain/create.xml",
            ])
            .build())
        }
    }

    let (mut client, mut connection) = connect_with_connector(
        FakeConnector,
        "test".into(),
        Duration::from_secs(5),
        Some(Duration::from_millis(100)),
    )
    .await
    .unwrap();
    tokio::spawn(async move {
        connection.run().await.unwrap();
        trace!("connection future resolved successfully")
    });

    trace!("Trying to get greeting");
    assert_eq!(
        client.xml_greeting().await.unwrap(),
        xml("response/greeting.xml")
    );

    let rsp = client
        .transact(
            &Login::new(
                "username",
                "password",
                Some("new-password"),
                Some(&["http://schema.ispapi.net/epp/xml/keyvalue-1.0"]),
            ),
            CLTRID,
        )
        .await
        .unwrap();
    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);
    trace!("Waiting");
    tokio::time::sleep(Duration::from_millis(250)).await;

    let contacts = &[
        DomainContact {
            contact_type: "admin".into(),
            id: "eppdev-contact-3".into(),
        },
        DomainContact {
            contact_type: "tech".into(),
            id: "eppdev-contact-3".into(),
        },
        DomainContact {
            contact_type: "billing".into(),
            id: "eppdev-contact-3".into(),
        },
    ];
    let create = DomainCreate::new(
        "eppdev-1.com",
        Period::years(1).unwrap(),
        None,
        Some("eppdev-contact-3"),
        "epP4uthd#v",
        Some(contacts),
    );
    trace!("Trying to create domains");
    let rsp = client.transact(&create, CLTRID).await.unwrap();
    assert_eq!(rsp.result.code, ResultCode::CommandCompletedSuccessfully);
}
