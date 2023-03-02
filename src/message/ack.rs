//! Types for EPP message ack request

use instant_xml::ToXml;

use crate::common::{NoExtension, EPP_XMLNS};
use crate::request::{Command, Transaction};

impl<'a> Transaction<NoExtension> for MessageAck<'a> {}

impl<'a> Command for MessageAck<'a> {
    type Response = String;
    const COMMAND: &'static str = "poll";
}

#[derive(Debug, ToXml)]
/// Type for EPP XML `<poll>` command for message ack
#[xml(rename = "poll", ns(EPP_XMLNS))]
pub struct MessageAck<'a> {
    /// The type of operation to perform
    /// The value is "ack" for message acknowledgement
    #[xml(attribute)]
    op: &'a str,
    /// The ID of the message to be acknowledged
    #[xml(rename = "msgID", attribute)]
    message_id: &'a str,
}

impl<'a> MessageAck<'a> {
    pub fn new(message_id: &'a str) -> Self {
        Self {
            op: "ack",
            message_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageAck;
    use crate::response::ResultCode;
    use crate::tests::{assert_serialized, response_from_file, SUCCESS_MSG, SVTRID};

    #[test]
    fn command() {
        let object = MessageAck::new("12345");
        assert_serialized("request/message/ack.xml", &object);
    }

    #[test]
    fn response() {
        let object = response_from_file::<MessageAck>("response/message/ack.xml");
        let msg = object.message_queue().unwrap();

        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);
        assert_eq!(object.result.message, SUCCESS_MSG);
        assert_eq!(msg.count, 4);
        assert_eq!(msg.id, "12345".to_string());
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
