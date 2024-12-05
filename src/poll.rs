use instant_xml::{FromXml, ToXml};

use crate::common::{NoExtension, EPP_XMLNS};
use crate::domain;
use crate::domain::transfer::TransferData;
use crate::extensions::low_balance::LowBalance;
use crate::extensions::rgp::poll::RgpPollData;
use crate::host;
use crate::request::{Command, Transaction};

impl Transaction<NoExtension> for Poll {}

impl Command for Poll {
    type Response = PollData;
    const COMMAND: &'static str = "poll";
}

impl Transaction<NoExtension> for Ack<'_> {}

impl Command for Ack<'_> {
    type Response = String;
    const COMMAND: &'static str = "poll";
}

// Request

/// Type for EPP XML `<poll>` command with `op="req"`
#[derive(Debug)]
pub struct Poll;

impl ToXml for Poll {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        serializer.write_start("poll", EPP_XMLNS)?;
        serializer.write_attr("op", EPP_XMLNS, &"req")?;
        serializer.end_empty()
    }
}

/// Type for EPP XML `<poll>` command with `op="ack"`
#[derive(Debug)]
pub struct Ack<'a> {
    /// The ID of the message to be acknowledged
    pub message_id: &'a str,
}

impl ToXml for Ack<'_> {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        serializer.write_start("poll", EPP_XMLNS)?;
        serializer.write_attr("op", EPP_XMLNS, &"ack")?;
        serializer.write_attr("msgID", EPP_XMLNS, &self.message_id)?;
        serializer.end_empty()
    }
}

// Response

/// Type that represents the `<resData>` tag for message poll response
#[derive(Debug, FromXml)]
#[xml(forward)]
pub enum PollData {
    /// Data under the `<domain:trnData>` tag
    DomainTransfer(TransferData),
    /// Data under the `<domain:infData>` tag
    DomainInfo(domain::InfoData),
    /// Data under the `<host:infData>` tag
    HostInfo(host::InfoData),
    /// Data under the `<lowbalance>` tag
    LowBalance(LowBalance),
    /// Data under the `<rgp-poll:pollData>` tag
    RgpPoll(RgpPollData),
}

#[cfg(test)]
mod tests {
    use super::{Ack, Poll, PollData};
    use crate::host::Status;
    use crate::response::ResultCode;
    use crate::tests::{assert_serialized, response_from_file, CLTRID, SUCCESS_MSG, SVTRID};

    use chrono::{TimeZone, Utc};
    use std::net::IpAddr;

    #[test]
    fn ack_command() {
        let object = Ack {
            message_id: "12345",
        };
        assert_serialized("request/poll/ack.xml", &object);
    }

    #[test]
    fn response() {
        let object = response_from_file::<Ack>("response/poll/ack.xml");
        let msg = object.message_queue().unwrap();

        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);
        assert_eq!(object.result.message, SUCCESS_MSG);
        assert_eq!(msg.count, 4);
        assert_eq!(msg.id, "12345".to_string());
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn poll_command() {
        let object = Poll;
        assert_serialized("request/poll/poll.xml", &object);
    }

    #[test]
    fn domain_transfer_response() {
        let object = response_from_file::<Poll>("response/poll/poll_domain_transfer.xml");
        let result = object.res_data().unwrap();
        let msg = object.message_queue().unwrap();

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );
        assert_eq!(msg.count, 5);
        assert_eq!(msg.id, "12345".to_string());
        assert_eq!(
            msg.date,
            Utc.with_ymd_and_hms(2021, 7, 23, 19, 12, 43).single()
        );
        assert_eq!(msg.message.as_ref().unwrap().text, "Transfer requested.");

        if let PollData::DomainTransfer(tr) = &result {
            assert_eq!(tr.name, "eppdev-transfer.com");
            assert_eq!(tr.transfer_status, "pending");
            assert_eq!(tr.requester_id, "eppdev");
            assert_eq!(
                tr.requested_at,
                Utc.with_ymd_and_hms(2021, 7, 23, 15, 31, 21).unwrap()
            );
            assert_eq!(tr.ack_id, "ClientY");
            assert_eq!(
                tr.ack_by,
                Utc.with_ymd_and_hms(2021, 7, 28, 15, 31, 21).unwrap()
            );
            assert_eq!(
                tr.expiring_at,
                Utc.with_ymd_and_hms(2022, 7, 2, 14, 53, 19).single()
            );
        } else {
            panic!("Wrong type");
        }

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn host_info_response() {
        let object = response_from_file::<Poll>("response/poll/poll_host_info.xml");
        let result = object.res_data().unwrap();
        let msg = object.message_queue().unwrap();

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );
        assert_eq!(msg.count, 4);
        assert_eq!(msg.id, "12345".to_string());
        assert_eq!(
            msg.date,
            Utc.with_ymd_and_hms(2022, 1, 2, 11, 30, 45).single()
        );
        assert_eq!(msg.message.as_ref().unwrap().text, "Unused objects policy");

        if let PollData::HostInfo(host) = &result {
            assert_eq!(host.name, "ns.test.com");

            assert_eq!(host.roid, "1234");
            assert!(host.statuses.iter().any(|&s| s == Status::Ok));
            assert!(host
                .addresses
                .iter()
                .any(|a| a == &IpAddr::from([1, 1, 1, 1])));
            assert_eq!(host.client_id, "1234");
            assert_eq!(host.creator_id, "user");
            assert_eq!(
                host.created_at,
                Utc.with_ymd_and_hms(2021, 12, 1, 22, 40, 48).unwrap()
            );
            assert_eq!(host.updater_id, Some("user".into()));
            assert_eq!(
                host.updated_at,
                Utc.with_ymd_and_hms(2021, 12, 1, 22, 40, 48).single()
            );
        } else {
            panic!("Wrong type");
        }

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn message_only_response() {
        let object = response_from_file::<Poll>("response/poll/poll_message_only.xml");
        let msg = object.message_queue().unwrap();
        dbg!(&msg);

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        assert_eq!(msg.count, 4);
        assert_eq!(msg.id, "12346".to_string());
        assert_eq!(
            msg.date,
            Utc.with_ymd_and_hms(2000, 6, 8, 22, 10, 0).single()
        );
        assert_eq!(msg.message.as_ref().unwrap().text, "Credit balance low.");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn empty_queue_response() {
        let object = response_from_file::<Poll>("response/poll/poll_empty_queue.xml");

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyNoMessages
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; no messages"
        );

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
