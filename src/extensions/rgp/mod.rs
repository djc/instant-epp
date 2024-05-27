//! Mapping for the Registry Grace Period extension
//!
//! As described in [RFC 3915](https://tools.ietf.org/html/rfc3915).

use instant_xml::FromXml;

pub mod poll; // Technically a separate extension (different namespace, RFC)
pub mod report;
pub mod request;

#[derive(Debug, PartialEq)]
pub enum RgpStatus {
    AddPeriod,
    AutoRenewPeriod,
    RenewPeriod,
    TransferPeriod,
    RedemptionPeriod,
    PendingRestore,
    PendingDelete,
}

impl<'xml> FromXml<'xml> for RgpStatus {
    #[inline]
    fn matches(id: ::instant_xml::Id<'_>, _: Option<::instant_xml::Id<'_>>) -> bool {
        id == ::instant_xml::Id {
            ns: XMLNS,
            name: "rgpStatus",
        } || id
            == ::instant_xml::Id {
                ns: poll::XMLNS,
                name: "rgpStatus",
            }
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
    ) -> Result<(), ::instant_xml::Error> {
        use ::instant_xml::{Error, Id};
        use instant_xml::de::Node;

        let node = match deserializer.next() {
            Some(result) => result?,
            None => return Err(Error::MissingValue(field)),
        };

        let attr = match node {
            Node::Attribute(attr) => attr,
            Node::Open(_) | Node::Text(_) => return Err(Error::MissingValue(field)),
            node => return Err(Error::UnexpectedNode(format!("{node:?} in RgpStatus"))),
        };

        let id = deserializer.attribute_id(&attr)?;
        let expected = Id { ns: "", name: "s" };
        if id != expected {
            return Err(Error::MissingValue(field));
        }

        *into = Some(match attr.value.as_ref() {
            "addPeriod" => Self::AddPeriod,
            "autoRenewPeriod" => Self::AutoRenewPeriod,
            "renewPeriod" => Self::RenewPeriod,
            "transferPeriod" => Self::TransferPeriod,
            "redemptionPeriod" => Self::RedemptionPeriod,
            "pendingRestore" => Self::PendingRestore,
            "pendingDelete" => Self::PendingDelete,
            val => {
                return Err(Error::UnexpectedValue(format!(
                    "invalid RgpStatus '{val:?}'"
                )))
            }
        });

        deserializer.ignore()?;
        Ok(())
    }

    type Accumulator = Option<Self>;

    const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
}

pub const XMLNS: &str = "urn:ietf:params:xml:ns:rgp-1.0";
