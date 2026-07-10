//! Types for EPP domain check request

use std::fmt;

use instant_xml::{FromXml, Serializer, ToXml};

use super::XMLNS;
use crate::common::{NoExtension, EPP_XMLNS};
use crate::request::{Command, Transaction};

impl Transaction<NoExtension> for DomainCheck<'_> {}

impl Command for DomainCheck<'_> {
    type Response = CheckData;
    const COMMAND: &'static str = "check";
}

// Request

#[derive(Debug, ToXml)]
#[xml(rename = "check", ns(XMLNS))]
struct DomainList<'a> {
    #[xml(rename = "name", ns(XMLNS))]
    domains: &'a [&'a str],
}

fn serialize_domains<W: fmt::Write + ?Sized>(
    domains: &[&str],
    serializer: &mut Serializer<W>,
) -> Result<(), instant_xml::Error> {
    DomainList { domains }.serialize(None, serializer)
}

#[derive(ToXml, Debug)]
#[xml(rename = "check", ns(EPP_XMLNS))]
pub struct DomainCheck<'a> {
    /// The list of domains to be checked for availability
    #[xml(serialize_with = "serialize_domains")]
    pub domains: &'a [&'a str],
}

// Response

#[derive(Debug, FromXml)]
#[xml(rename = "name", ns(XMLNS))]
pub struct Name {
    #[xml(attribute, rename = "avail")]
    pub available: bool,
    #[xml(direct)]
    pub value: String,
}

#[derive(Debug, FromXml)]
#[xml(rename = "cd", ns(XMLNS))]
pub struct CheckedDomain {
    /// Data under the `<cd>` tag
    pub name: Name,
    /// Data under the `<reason>` tag
    pub reason: Option<Reason>,
}

impl CheckedDomain {
    /// Returns a tuple of the availability and reason (if any) for the domain name
    pub fn available(&self) -> (bool, Option<&str>) {
        (
            self.name.available,
            self.reason.as_ref().map(|r| r.value.as_str()),
        )
    }

    /// Returns the domain name value
    pub fn name(&self) -> &str {
        &self.name.value
    }
}

#[derive(Debug, FromXml)]
#[xml(rename = "reason", ns(XMLNS))]
pub struct Reason {
    #[xml(attribute)]
    pub lang: Option<String>,
    #[xml(direct)]
    pub value: String,
}

/// Type that represents the `<chkData>` tag for host check response
#[derive(Debug, FromXml)]
#[xml(rename = "chkData", ns(XMLNS))]
pub struct CheckData {
    pub list: Vec<CheckedDomain>,
}

#[cfg(test)]
mod tests {
    use super::DomainCheck;
    use crate::response::ResultCode;
    use crate::tests::{assert_serialized, response_from_file, CLTRID, SUCCESS_MSG, SVTRID};

    #[test]
    fn command() {
        let object = DomainCheck {
            domains: &["eppdev.com", "eppdev.net"],
        };
        assert_serialized("request/domain/check.xml", &object);
    }

    #[test]
    fn response() {
        let object = response_from_file::<DomainCheck>("response/domain/check.xml");
        let result = dbg!(&object).res_data().unwrap();

        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);
        assert_eq!(object.result.message, SUCCESS_MSG);
        assert_eq!(result.list[0].name.value, "eppdev.com");
        assert!(result.list[0].name.available);
        assert_eq!(result.list[1].name.value, "eppdev.net");
        assert!(!result.list[1].name.available);
        assert!(!result.list[2].name.available);
        assert_eq!(result.list[2].reason.as_ref().unwrap().value, "In Use");
        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
