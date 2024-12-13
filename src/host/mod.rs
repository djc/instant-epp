//! Mapping for EPP host objects
//!
//! As described in [RFC 5732](https://tools.ietf.org/html/rfc5732).

use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;

use instant_xml::{Deserializer, FromXml, Serializer, ToXml};

pub mod check;
pub use check::HostCheck;

pub mod create;
pub use create::HostCreate;

pub mod delete;
pub use delete::HostDelete;

pub mod info;
pub use info::{HostInfo, InfoData};

pub mod update;
pub use update::HostUpdate;

pub const XMLNS: &str = "urn:ietf:params:xml:ns:host-1.0";

/// The `<status>` type on contact transactions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    ClientDeleteProhibited,
    ServerDeleteProhibited,
    ClientUpdateProhibited,
    ServerUpdateProhibited,
    Linked,
    Ok,
    PendingCreate,
    PendingDelete,
    PendingTransfer,
    PendingUpdate,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        use Status::*;
        match self {
            ClientDeleteProhibited => "clientDeleteProhibited",
            ServerDeleteProhibited => "serverDeleteProhibited",
            ClientUpdateProhibited => "clientUpdateProhibited",
            ServerUpdateProhibited => "serverUpdateProhibited",
            Linked => "linked",
            Ok => "ok",
            PendingCreate => "pendingCreate",
            PendingDelete => "pendingDelete",
            PendingTransfer => "pendingTransfer",
            PendingUpdate => "pendingUpdate",
        }
    }
}

impl ToXml for Status {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        _: Option<instant_xml::Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        serializer.write_start("status", XMLNS)?;
        serializer.write_attr("s", XMLNS, &self.as_str())?;
        serializer.end_empty()
    }
}

impl<'xml> FromXml<'xml> for Status {
    fn matches(id: instant_xml::Id<'_>, _: Option<instant_xml::Id<'_>>) -> bool {
        id == instant_xml::Id {
            ns: XMLNS,
            name: "status",
        }
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), instant_xml::Error> {
        use instant_xml::de::Node;
        use instant_xml::{Error, Id};

        let node = match deserializer.next() {
            Some(result) => result?,
            None => return Err(Error::MissingValue(field)),
        };

        let attr = match node {
            Node::Attribute(attr) => attr,
            Node::Open(_) | Node::Text(_) => return Err(Error::MissingValue(field)),
            node => return Err(Error::UnexpectedNode(format!("{node:?} in Status"))),
        };

        let id = deserializer.attribute_id(&attr)?;
        let expected = Id { ns: "", name: "s" };
        if id != expected {
            return Err(Error::MissingValue(field));
        }

        *into = Some(match attr.value.as_ref() {
            "clientDeleteProhibited" => Self::ClientDeleteProhibited,
            "serverDeleteProhibited" => Self::ServerDeleteProhibited,
            "clientUpdateProhibited" => Self::ClientUpdateProhibited,
            "serverUpdateProhibited" => Self::ServerUpdateProhibited,
            "linked" => Self::Linked,
            "ok" => Self::Ok,
            "pendingCreate" => Self::PendingCreate,
            "pendingDelete" => Self::PendingDelete,
            "pendingTransfer" => Self::PendingTransfer,
            "pendingUpdate" => Self::PendingUpdate,
            val => return Err(Error::UnexpectedValue(format!("invalid status {val:?}"))),
        });

        deserializer.ignore()?;
        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: instant_xml::Kind = instant_xml::Kind::Element;
}

/// The `<hostAddr>` types domain or host transactions
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "addr", ns(XMLNS))]
struct HostAddr<'a> {
    #[xml(attribute, rename = "ip")]
    ip_version: Option<Cow<'a, str>>,
    #[xml(direct)]
    address: Cow<'a, str>,
}

impl From<&IpAddr> for HostAddr<'static> {
    fn from(addr: &IpAddr) -> Self {
        Self {
            ip_version: Some(match addr {
                IpAddr::V4(_) => "v4".into(),
                IpAddr::V6(_) => "v6".into(),
            }),
            address: addr.to_string().into(),
        }
    }
}

pub(crate) fn serialize_host_addrs_option<T: AsRef<[IpAddr]>, W: fmt::Write + ?Sized>(
    addrs: &Option<T>,
    serializer: &mut Serializer<'_, W>,
) -> Result<(), instant_xml::Error> {
    let addrs = match addrs {
        Some(addrs) => addrs.as_ref(),
        None => return Ok(()),
    };

    for addr in addrs {
        HostAddr::from(addr).serialize(None, serializer)?;
    }

    Ok(())
}
