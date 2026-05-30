use std::time::Duration;

use async_trait::async_trait;
use tokio_test::io::Builder;

use instant_epp::client::{Connector, EppClient};
use instant_epp::Error;

#[tokio::test]
async fn greeting_single_chunk() {
    assert!(connect_with_chunks(1).await.is_ok());
}

#[tokio::test]
async fn greeting_two_chunks() {
    assert!(connect_with_chunks(2).await.is_ok());
}

async fn connect_with_chunks(num_chunks: usize) -> Result<EppClient<impl Connector>, Error> {
    struct FakeConnector {
        num_chunks: usize,
    }

    #[async_trait]
    impl Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, Error> {
            let mut builder = Builder::new();

            builder.read(&((GREETING.len() as u32) + 4).to_be_bytes());

            let chunk_size = GREETING.len() / self.num_chunks;
            for i in 0..self.num_chunks {
                let start = i * chunk_size;
                let end = if i == self.num_chunks - 1 {
                    GREETING.len()
                } else {
                    start + chunk_size
                };
                builder.read(&GREETING[start..end]);
            }

            Ok(builder.build())
        }
    }

    EppClient::new(
        FakeConnector { num_chunks },
        "test".into(),
        Duration::from_secs(5),
    )
    .await
}

const GREETING: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<epp xmlns="urn:ietf:params:xml:ns:epp-1.0">
  <greeting>
    <svID>Test EPP Server</svID>
    <svDate>2024-01-01T00:00:00Z</svDate>
    <svcMenu>
      <version>1.0</version>
      <lang>en</lang>
      <objURI>urn:ietf:params:xml:ns:domain-1.0</objURI>
    </svcMenu>
  </greeting>
</epp>"#;
