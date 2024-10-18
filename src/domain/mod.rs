//! Mapping for EPP domain objects
//!
//! As described in [RFC 5731](https://tools.ietf.org/html/rfc5731).

use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

use instant_xml::OptionAccumulator;
use instant_xml::{Accumulate, Deserializer, FromXml, Serializer, ToXml};

use crate::Error;

pub mod check;
pub use check::DomainCheck;

pub mod create;
pub use create::DomainCreate;

pub mod delete;
pub use delete::DomainDelete;

pub mod info;
pub use info::DomainInfo;

pub mod renew;
pub use renew::DomainRenew;

pub mod transfer;
pub use transfer::DomainTransfer;

pub mod update;
pub use update::DomainUpdate;

pub const XMLNS: &str = "urn:ietf:params:xml:ns:domain-1.0";

/// The `<hostAttr>` type for domain transactions
#[derive(Clone, Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(rename = "hostAttr", ns(XMLNS))]
pub struct HostAttr<'a> {
    /// The `<hostName>` tag
    #[xml(rename = "hostName")]
    pub name: Cow<'a, str>,
    /// The `<hostAddr>` tags
    #[xml(
        rename = "hostAddr",
        serialize_with = "serialize_host_addrs_option",
        deserialize_with = "deserialize_host_addrs_option"
    )]
    pub addresses: Option<Vec<IpAddr>>,
}

fn deserialize_host_addrs_option<'xml>(
    into: &mut OptionAccumulator<Vec<IpAddr>, Vec<IpAddr>>,
    field: &'static str,
    deserializer: &mut Deserializer<'_, 'xml>,
) -> Result<(), instant_xml::Error> {
    let mut value = <Option<Vec<HostAddr<'static>>> as FromXml<'xml>>::Accumulator::default();
    <Option<Vec<HostAddr<'static>>>>::deserialize(&mut value, field, deserializer)?;
    let new = match value.try_done(field)? {
        Some(new) => new,
        None => return Ok(()),
    };

    let into = into.get_mut();
    for addr in new {
        match IpAddr::from_str(&addr.address) {
            Ok(ip) => into.push(ip),
            Err(_) => {
                return Err(instant_xml::Error::UnexpectedValue(format!(
                    "invalid IP address '{}'",
                    &addr.address
                )))
            }
        }
    }

    Ok(())
}

/// The `<hostAddr>` types domain or host transactions
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "hostAddr", ns(super::domain::XMLNS))]
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

#[derive(Clone, Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(rename = "hostObj", ns(XMLNS))]
pub struct HostObj<'a> {
    #[xml(direct)]
    pub name: Cow<'a, str>,
}

#[derive(Clone, Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(forward)]
pub enum HostInfo<'a> {
    Attr(HostAttr<'a>),
    Obj(HostObj<'a>),
}

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "ns", ns(XMLNS))]
pub struct NameServers<'a> {
    pub ns: Cow<'a, [HostInfo<'a>]>,
}

/// The `<contact>` type on domain creation and update requests
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "contact", ns(XMLNS))]
pub struct DomainContact<'a> {
    /// The contact type attr (usually admin, billing, or tech in most registries)
    #[xml(attribute, rename = "type")]
    pub contact_type: Cow<'a, str>,
    /// The contact id
    #[xml(direct)]
    pub id: Cow<'a, str>,
}

/// The `<period>` type for registration, renewal or transfer on domain transactions
#[derive(Clone, Copy, Debug)]
pub struct Period {
    /// The interval (usually 'y' indicating years)
    unit: PerdiodUnit,
    /// The length of the registration, renewal or transfer period (usually in years)
    length: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum PerdiodUnit {
    Years,
    Months,
}

impl Period {
    pub fn years(length: u8) -> Result<Self, Error> {
        Self::new(length, PerdiodUnit::Years)
    }

    pub fn months(length: u8) -> Result<Self, Error> {
        Self::new(length, PerdiodUnit::Months)
    }

    pub fn new(length: u8, unit: PerdiodUnit) -> Result<Self, Error> {
        match length {
            1..=99 => Ok(Self { length, unit }),
            0 | 100.. => Err(Error::Other(
                "Period length must be greater than 0 and less than 100".into(),
            )),
        }
    }
}

impl ToXml for Period {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<instant_xml::Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        #[derive(ToXml)]
        #[xml(rename = "period", ns(XMLNS))]
        struct PeriodXml {
            /// The interval (usually 'y' indicating years)
            #[xml(attribute)]
            unit: char,
            /// The length of the registration, renewal or transfer period (usually in years)
            #[xml(direct)]
            length: u8,
        }

        let period = PeriodXml {
            unit: match self.unit {
                PerdiodUnit::Years => 'y',
                PerdiodUnit::Months => 'm',
            },
            length: self.length,
        };

        period.serialize(field, serializer)
    }
}

pub const ONE_YEAR: Period = Period {
    unit: PerdiodUnit::Years,
    length: 1,
};

pub const TWO_YEARS: Period = Period {
    unit: PerdiodUnit::Years,
    length: 2,
};

pub const THREE_YEARS: Period = Period {
    unit: PerdiodUnit::Years,
    length: 3,
};

pub const ONE_MONTH: Period = Period {
    unit: PerdiodUnit::Months,
    length: 1,
};

pub const SIX_MONTHS: Period = Period {
    unit: PerdiodUnit::Months,
    length: 6,
};

/// The `<authInfo>` tag for domain and contact transactions
#[derive(Clone, Debug, FromXml, ToXml)]
#[xml(rename = "authInfo", ns(XMLNS))]
pub struct DomainAuthInfo<'a> {
    /// The `<pw>` tag under `<authInfo>`
    #[xml(rename = "pw")]
    pub password: Cow<'a, str>,
}

impl<'a> DomainAuthInfo<'a> {
    /// Creates a DomainAuthInfo instance with the given password
    pub fn new(password: &'a str) -> Self {
        Self {
            password: password.into(),
        }
    }
}

/// The `<status>` type on contact transactions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    ClientDeleteProhibited,
    ServerDeleteProhibited,
    ClientHold,
    ServerHold,
    ClientRenewProhibited,
    ServerRenewProhibited,
    ClientTransferProhibited,
    ServerTransferProhibited,
    ClientUpdateProhibited,
    ServerUpdateProhibited,
    Inactive,
    Ok,
    PendingCreate,
    PendingDelete,
    PendingRenew,
    PendingTransfer,
    PendingUpdate,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        use Status::*;
        match self {
            ClientDeleteProhibited => "clientDeleteProhibited",
            ServerDeleteProhibited => "serverDeleteProhibited",
            ClientHold => "clientHold",
            ServerHold => "serverHold",
            ClientRenewProhibited => "clientRenewProhibited",
            ServerRenewProhibited => "serverRenewProhibited",
            ClientTransferProhibited => "clientTransferProhibited",
            ServerTransferProhibited => "serverTransferProhibited",
            ClientUpdateProhibited => "clientUpdateProhibited",
            ServerUpdateProhibited => "serverUpdateProhibited",
            Inactive => "inactive",
            Ok => "ok",
            PendingCreate => "pendingCreate",
            PendingDelete => "pendingDelete",
            PendingRenew => "pendingRenew",
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
            "clientHold" => Self::ClientHold,
            "serverHold" => Self::ServerHold,
            "clientRenewProhibited" => Self::ClientRenewProhibited,
            "serverRenewProhibited" => Self::ServerRenewProhibited,
            "clientTransferProhibited" => Self::ClientTransferProhibited,
            "serverTransferProhibited" => Self::ServerTransferProhibited,
            "clientUpdateProhibited" => Self::ClientUpdateProhibited,
            "serverUpdateProhibited" => Self::ServerUpdateProhibited,
            "inactive" => Self::Inactive,
            "ok" => Self::Ok,
            "pendingCreate" => Self::PendingCreate,
            "pendingDelete" => Self::PendingDelete,
            "pendingRenew" => Self::PendingRenew,
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
