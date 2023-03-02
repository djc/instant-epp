//! Mapping for EPP contact objects
//!
//! As described in [RFC 5733](https://tools.ietf.org/html/rfc5733).

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use instant_xml::{display_to_xml, from_xml_str, FromXml, ToXml};

pub mod check;
pub use check::ContactCheck;

pub mod create;
pub use create::ContactCreate;

pub mod delete;
pub use delete::ContactDelete;

pub mod info;
pub use info::ContactInfo;

pub mod update;
pub use update::ContactUpdate;

pub const XMLNS: &str = "urn:ietf:params:xml:ns:contact-1.0";

#[derive(Debug, Clone)]
pub struct Country(celes::Country);

impl<'xml> FromXml<'xml> for Country {
    fn matches(id: instant_xml::Id<'_>, _: Option<instant_xml::Id<'_>>) -> bool {
        id == instant_xml::Id {
            ns: XMLNS,
            name: "cc",
        }
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut instant_xml::Deserializer<'cx, 'xml>,
    ) -> Result<(), instant_xml::Error> {
        from_xml_str(deserializer, into)
    }

    type Accumulator = Option<Self>;
    const KIND: instant_xml::Kind = instant_xml::Kind::Scalar;
}

impl ToXml for Country {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        display_to_xml(&self.0.alpha2, field, serializer)
    }
}

impl FromStr for Country {
    type Err = <celes::Country as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(celes::Country::from_str(s)?))
    }
}

impl std::ops::Deref for Country {
    type Target = celes::Country;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The `<authInfo>` tag for domain and contact transactions
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "authInfo", ns(XMLNS))]
pub struct ContactAuthInfo<'a> {
    /// The `<pw>` tag under `<authInfo>`
    #[xml(rename = "pw")]
    pub password: Cow<'a, str>,
}

impl<'a> ContactAuthInfo<'a> {
    /// Creates a ContactAuthInfo instance with the given password
    pub fn new(password: &'a str) -> Self {
        Self {
            password: password.into(),
        }
    }
}

/// The data for `<voice>` types on domain transactions
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "voice", ns(XMLNS))]
pub struct Voice<'a> {
    /// The value of the 'x' attr on `<voice>` and `<fax>` tags
    #[xml(rename = "x", attribute)]
    pub extension: Option<Cow<'a, str>>,
    /// The inner text on the `<voice>` and `<fax>` tags
    #[xml(direct)]
    pub number: Cow<'a, str>,
}

impl<'a> Voice<'a> {
    /// Creates a new Phone instance with a given phone number
    pub fn new(number: &'a str) -> Self {
        Self {
            extension: None,
            number: number.into(),
        }
    }

    /// Sets the extension value of the Phone type
    pub fn set_extension(&mut self, ext: &'a str) {
        self.extension = Some(ext.into());
    }
}

/// The data for `<voice>` and `<fax>` types on domain transactions
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "fax", ns(XMLNS))]
pub struct Fax<'a> {
    /// The value of the 'x' attr on `<voice>` and `<fax>` tags
    #[xml(rename = "x", attribute)]
    pub extension: Option<Cow<'a, str>>,
    /// The inner text on the `<voice>` and `<fax>` tags
    #[xml(direct)]
    pub number: Cow<'a, str>,
}

impl<'a> Fax<'a> {
    /// Creates a new Phone instance with a given phone number
    pub fn new(number: &'a str) -> Self {
        Self {
            extension: None,
            number: number.into(),
        }
    }

    /// Sets the extension value of the Phone type
    pub fn set_extension(&mut self, ext: &'a str) {
        self.extension = Some(ext.into());
    }
}

/// The `<addr>` type on contact transactions
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "addr", ns(XMLNS))]
pub struct Address<'a> {
    /// The `<street>` tags under `<addr>`
    pub street: Vec<Cow<'a, str>>,
    /// The `<city>` tag under `<addr>`
    pub city: Cow<'a, str>,
    /// The `<sp>` tag under `<addr>`
    #[xml(rename = "sp")]
    pub province: Cow<'a, str>,
    /// The `<pc>` tag under `<addr>`
    #[xml(rename = "pc")]
    pub postal_code: Cow<'a, str>,
    /// The `<cc>` tag under `<addr>`
    #[xml(rename = "cc")]
    pub country: Country,
}

impl<'a> Address<'a> {
    /// Creates a new Address instance
    pub fn new(
        street: &[&'a str],
        city: &'a str,
        province: &'a str,
        postal_code: &'a str,
        country: Country,
    ) -> Self {
        let street = street.iter().map(|&s| s.into()).collect();

        Self {
            street,
            city: city.into(),
            province: province.into(),
            postal_code: postal_code.into(),
            country,
        }
    }
}

/// The `<postalInfo>` type on contact transactions
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "postalInfo", ns(XMLNS))]
pub struct PostalInfo<'a> {
    /// The 'type' attr on `<postalInfo>`
    #[xml(rename = "type", attribute)]
    pub info_type: Cow<'a, str>,
    /// The `<name>` tag under `<postalInfo>`
    pub name: Cow<'a, str>,
    /// The `<org>` tag under `<postalInfo>`
    #[xml(rename = "org")]
    pub organization: Cow<'a, str>,
    /// The `<addr>` tag under `<postalInfo>`
    pub address: Address<'a>,
}

impl<'a> PostalInfo<'a> {
    /// Creates a new PostalInfo instance
    pub fn new(
        info_type: &'a str,
        name: &'a str,
        organization: &'a str,
        address: Address<'a>,
    ) -> Self {
        Self {
            info_type: info_type.into(),
            name: name.into(),
            organization: organization.into(),
            address,
        }
    }
}

/// The `<status>` type on contact transactions
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "status", ns(XMLNS))]
pub struct Status<'a> {
    /// The status name, represented by the 's' attr on `<status>` tags
    #[xml(attribute, rename = "s")]
    pub status: Cow<'a, str>,
}
