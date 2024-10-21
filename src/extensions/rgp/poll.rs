//! <https://www.verisign.com/assets/epp-sdk/verisign_epp-extension_rgp-poll_v00.html>

use chrono::{DateTime, Utc};
use instant_xml::FromXml;

use super::RgpStatus;

/// RGP request status
#[derive(Debug, FromXml)]
#[xml(rename = "pollData", ns(XMLNS), rename_all = "camelCase")]
pub struct RgpPollData {
    pub name: String,
    pub rgp_status: RgpStatus,
    pub req_date: DateTime<Utc>,
    pub report_due_date: DateTime<Utc>,
}

pub(super) const XMLNS: &str = "http://www.verisign.com/epp/rgp-poll-1.0";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poll::{Poll, PollData};
    use crate::tests::response_from_file;

    #[test]
    fn rgp_poll_data() {
        let object = response_from_file::<Poll>("response/poll/poll_rgp_restore.xml");
        let Some(PollData::RgpPoll(data)) = object.res_data() else {
            panic!("expected RgpPollData");
        };

        assert_eq!(data.name, "EXAMPLE.COM");
        assert_eq!(data.rgp_status, RgpStatus::PendingRestore);
    }
}
