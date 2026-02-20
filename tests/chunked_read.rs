use std::time::Duration;

use async_trait::async_trait;
use rstest::rstest;
use tokio_test::io::Builder;

use instant_epp::client::{Connector, EppClient};
use instant_epp::Error;

fn len_bytes(bytes: &[u8]) -> [u8; 4] {
    ((bytes.len() as u32) + 4).to_be_bytes()
}

fn greeting() -> &'static [u8] {
    br#"<?xml version="1.0" encoding="UTF-8"?>
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
</epp>"#
}

#[rstest]
#[case::single_chunk(1)]
#[case::two_chunks(2)]
#[tokio::test]
async fn test_chunked_greeting(#[case] num_chunks: usize) {
    struct FakeConnector {
        num_chunks: usize,
    }

    #[async_trait]
    impl Connector for FakeConnector {
        type Connection = tokio_test::io::Mock;

        async fn connect(&self, _: Duration) -> Result<Self::Connection, Error> {
            let mut builder = Builder::new();
            let buf = greeting();

            builder.read(&len_bytes(buf));

            let chunk_size = buf.len() / self.num_chunks;
            for i in 0..self.num_chunks {
                let start = i * chunk_size;
                let end = if i == self.num_chunks - 1 {
                    buf.len()
                } else {
                    start + chunk_size
                };
                builder.read(&buf[start..end]);
            }

            Ok(builder.build())
        }
    }

    let connector = FakeConnector { num_chunks };
    let result = EppClient::new(connector, "test".into(), Duration::from_secs(5)).await;

    assert!(
        result.is_ok(),
        "Failed to read greeting in {} chunk(s)",
        num_chunks
    );
}
