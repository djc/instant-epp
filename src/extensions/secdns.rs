//! DNS security extensions mapping
//!
//! As described in [RFC 5910](https://www.rfc-editor.org/rfc/rfc5910)
use instant_xml::{Error, Id, Serializer, ToXml};
use std::borrow::Cow;
use std::fmt::Write;
use std::time::Duration;

use crate::common::NoExtension;
use crate::request::{Extension, Transaction};

pub const XMLNS: &str = "urn:ietf:params:xml:ns:secDNS-1.1";

impl<'a> Transaction<CreateData<'a>> for crate::domain::create::DomainCreate<'a> {}

impl Extension for CreateData<'_> {
    type Response = NoExtension;
}

#[derive(Debug, ToXml)]
#[xml(rename = "create", ns(XMLNS))]
pub struct CreateData<'a> {
    data: DsOrKeyType<'a>,
}

impl<'a> From<&'a [DsDataType<'a>]> for CreateData<'a> {
    fn from(s: &'a [DsDataType<'a>]) -> Self {
        Self {
            data: DsOrKeyType {
                maximum_signature_lifetime: None,
                data: DsOrKeyData::DsData(s),
            },
        }
    }
}

impl<'a> From<&'a [KeyDataType<'a>]> for CreateData<'a> {
    fn from(s: &'a [KeyDataType<'a>]) -> Self {
        Self {
            data: DsOrKeyType {
                maximum_signature_lifetime: None,
                data: DsOrKeyData::KeyData(s),
            },
        }
    }
}

impl<'a> From<(Duration, &'a [DsDataType<'a>])> for CreateData<'a> {
    fn from((maximum_signature_lifetime, data): (Duration, &'a [DsDataType<'a>])) -> Self {
        Self {
            data: DsOrKeyType {
                maximum_signature_lifetime: Some(maximum_signature_lifetime),
                data: DsOrKeyData::DsData(data),
            },
        }
    }
}

impl<'a> From<(Duration, &'a [KeyDataType<'a>])> for CreateData<'a> {
    fn from((maximum_signature_lifetime, data): (Duration, &'a [KeyDataType<'a>])) -> Self {
        Self {
            data: DsOrKeyType {
                maximum_signature_lifetime: Some(maximum_signature_lifetime),
                data: DsOrKeyData::KeyData(data),
            },
        }
    }
}

/// Struct supporting either the `dsData` or the `keyData` interface.
#[derive(Debug)]
pub struct DsOrKeyType<'a> {
    maximum_signature_lifetime: Option<Duration>,
    data: DsOrKeyData<'a>,
}

impl ToXml for DsOrKeyType<'_> {
    fn serialize<W: Write + ?Sized>(
        &self,
        _: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        if let Some(maximum_signature_lifetime) = self.maximum_signature_lifetime {
            let nc_name = "maxSigLife";
            let prefix = serializer.write_start(nc_name, XMLNS)?;
            serializer.end_start()?;
            maximum_signature_lifetime
                .as_secs()
                .serialize(None, serializer)?;
            serializer.write_close(prefix, nc_name)?;
        }
        match &self.data {
            DsOrKeyData::DsData(data) => data.serialize(None, serializer)?,
            DsOrKeyData::KeyData(data) => data.serialize(None, serializer)?,
        }
        Ok(())
    }
}

#[derive(Debug, ToXml)]
#[xml(forward)]
pub enum DsOrKeyData<'a> {
    DsData(&'a [DsDataType<'a>]),
    KeyData(&'a [KeyDataType<'a>]),
}

#[derive(Debug, ToXml)]
#[xml(rename = "dsData", ns(XMLNS))]
pub struct DsDataType<'a> {
    #[xml(rename = "keyTag")]
    key_tag: u16,
    #[xml(rename = "alg")]
    algorithm: Algorithm,
    #[xml(rename = "digestType")]
    digest_type: DigestAlgorithm,
    digest: Cow<'a, str>,
    #[xml(rename = "keyData")]
    key_data: Option<KeyDataType<'a>>,
}

impl<'a> DsDataType<'a> {
    pub fn new(
        key_tag: u16,
        algorithm: Algorithm,
        digest_type: DigestAlgorithm,
        digest: impl Into<Cow<'a, str>>,
        key_data: Option<KeyDataType<'a>>,
    ) -> Self {
        Self {
            key_tag,
            algorithm,
            digest_type,
            digest: digest.into(),
            key_data,
        }
    }
}

/// DigestAlgorithm identifies the algorithm used to construct the digest
/// <https://www.iana.org/assignments/ds-rr-types/ds-rr-types.xhtml>
#[derive(Clone, Copy, Debug)]
// XXX Do NOT derive PartialEq, Hash or Ord because the variant
// Other(u8) could clash with one of the other variants. They have to
// be hand coded.
pub enum DigestAlgorithm {
    Sha1,
    Sha256,
    Gost,
    Sha384,
    Other(u8),
}

impl From<DigestAlgorithm> for u8 {
    fn from(s: DigestAlgorithm) -> Self {
        match s {
            DigestAlgorithm::Sha1 => 1,
            DigestAlgorithm::Sha256 => 2,
            DigestAlgorithm::Gost => 3,
            DigestAlgorithm::Sha384 => 4,
            DigestAlgorithm::Other(n) => n,
        }
    }
}

impl ToXml for DigestAlgorithm {
    fn serialize<W: Write + ?Sized>(
        &self,
        id: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        u8::from(*self).serialize(id, serializer)
    }
}

/// Algorithm identifies the public key's cryptographic algorithm
/// <https://www.iana.org/assignments/dns-sec-alg-numbers/dns-sec-alg-numbers.xhtml#dns-sec-alg-numbers-1>
#[derive(Clone, Copy, Debug)]
// XXX Do NOT derive PartialEq, Hash or Ord because the variant
// Other(u8) could clash with one of the other variants. They have to
// be hand coded.
pub enum Algorithm {
    // Delete DS
    Delete,
    /// RSA/MD5
    RsaMd5,
    /// Diffie-Hellman
    Dh,
    /// DSA/SHA-1
    Dsa,
    /// Elliptic Curve
    Ecc,
    /// RSA/SHA-1
    RsaSha1,
    /// DSA-NSEC3-SHA1
    DsaNsec3Sha1,
    /// RSASHA1-NSEC3-SHA1
    RsaSha1Nsec3Sha1,
    /// RSA/SHA-256
    RsaSha256,
    /// RSA/SHA-512
    RsaSha512,
    /// GOST R 34.10-2001
    EccGost,
    /// ECDSA Curve P-256 with SHA-256
    EcdsaP256Sha256,
    /// ECDSA Curve P-384 with SHA-384
    EcdsaP384Sha384,
    /// Ed25519
    Ed25519,
    /// Ed448
    Ed448,
    /// Indirect
    Indirect,
    /// Private
    PrivateDns,
    /// Private
    PrivateOid,
    Other(u8),
}

impl From<Algorithm> for u8 {
    fn from(s: Algorithm) -> Self {
        match s {
            Algorithm::Delete => 0,
            Algorithm::RsaMd5 => 1,
            Algorithm::Dh => 2,
            Algorithm::Dsa => 3,
            // RFC 4034
            Algorithm::Ecc => 4,
            Algorithm::RsaSha1 => 5,
            Algorithm::DsaNsec3Sha1 => 6,
            Algorithm::RsaSha1Nsec3Sha1 => 7,
            Algorithm::RsaSha256 => 8,
            Algorithm::RsaSha512 => 10,
            Algorithm::EccGost => 12,
            Algorithm::EcdsaP256Sha256 => 13,
            Algorithm::EcdsaP384Sha384 => 14,
            Algorithm::Ed25519 => 15,
            Algorithm::Ed448 => 16,
            Algorithm::Indirect => 252,
            Algorithm::PrivateDns => 253,
            Algorithm::PrivateOid => 254,
            Algorithm::Other(n) => n,
        }
    }
}

impl ToXml for Algorithm {
    fn serialize<W: Write + ?Sized>(
        &self,
        id: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        u8::from(*self).serialize(id, serializer)
    }
}

#[derive(Debug, ToXml)]
#[xml(rename = "keyData", ns(XMLNS))]
pub struct KeyDataType<'a> {
    flags: Flags,
    protocol: Protocol,
    #[xml(rename = "alg")]
    algorithm: Algorithm,
    #[xml(rename = "pubKey")]
    public_key: Cow<'a, str>,
}

impl<'a> KeyDataType<'a> {
    pub fn new(
        flags: Flags,
        protocol: Protocol,
        algorithm: Algorithm,
        public_key: &'a str,
    ) -> Self {
        Self {
            flags,
            protocol,
            algorithm,
            public_key: public_key.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Flags {
    /// Zone Key flag. If `true` then the DNSKEY record holds a DNS
    /// zone key. If `false` then the DNSKEY record holds some other
    /// type of DNS public key.
    zone_key: bool,
    /// Secure Entry Point. If `true` then the DNSKEY record holds a
    /// key intended for use as a secure entry point.
    secure_entry_point: bool,
}

impl From<Flags> for u16 {
    fn from(flags: Flags) -> Self {
        let mut res = 0;
        if flags.zone_key {
            res |= 0b1_0000_0000;
        }
        if flags.secure_entry_point {
            res |= 0x1;
        }
        res
    }
}

impl ToXml for Flags {
    fn serialize<W: Write + ?Sized>(
        &self,
        id: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        u16::from(*self).serialize(id, serializer)
    }
}

/// `Flags` for a zone signing key.
pub const FLAGS_DNS_ZONE_KEY: Flags = Flags {
    zone_key: true,
    secure_entry_point: false,
};
/// `Flags` for a key signing key.
pub const FLAGS_DNS_ZONE_KEY_SEP: Flags = Flags {
    zone_key: true,
    secure_entry_point: true,
};

#[derive(Clone, Copy, Debug)]
// XXX Do NOT derive PartialEq, Hash or Ord because the variant
// Other(u8) could clash with one of the other variants. They have to
// be hand coded.
pub enum Protocol {
    /// RFC 2535, reserved
    Tls,
    /// RFC 2535, reserved
    Email,
    /// RFC 5034 DNSSEC
    Dnssec,
    /// RFC 2535, reserved
    Ipsec,
    /// RFC 2535
    All,
    Other(u8),
}

impl From<Protocol> for u8 {
    fn from(s: Protocol) -> Self {
        match s {
            Protocol::Tls => 1,
            Protocol::Email => 2,
            Protocol::Dnssec => 3,
            Protocol::Ipsec => 4,
            Protocol::All => 255,
            Protocol::Other(n) => n,
        }
    }
}

impl ToXml for Protocol {
    fn serialize<W: Write + ?Sized>(
        &self,
        id: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        u8::from(*self).serialize(id, serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{self, Period, PeriodLength};
    use crate::tests::assert_serialized;

    #[test]
    fn create_ds_data_interface() {
        let ds_data = [DsDataType::new(
            12345,
            Algorithm::Dsa,
            DigestAlgorithm::Sha1,
            "49FD46E6C4B45C55D4AC",
            None,
        )];
        let extension = CreateData::from((Duration::from_secs(604800), ds_data.as_ref()));
        let ns = [
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns1.example.com".into(),
            }),
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns2.example.com".into(),
            }),
        ];
        let contact = [
            domain::DomainContact {
                contact_type: "admin".into(),
                id: "sh8013".into(),
            },
            domain::DomainContact {
                contact_type: "tech".into(),
                id: "sh8013".into(),
            },
        ];
        let object = domain::DomainCreate::new(
            "example.com",
            Period::Years(PeriodLength::new(2).unwrap()),
            Some(&ns),
            Some("jd1234"),
            "2fooBAR",
            Some(&contact),
        );
        assert_serialized(
            "request/extensions/secdns_create_ds.xml",
            (&object, &extension),
        );
    }

    #[test]
    fn create_ds_and_key_data_interface() {
        let key_data = KeyDataType::new(
            FLAGS_DNS_ZONE_KEY_SEP,
            Protocol::Dnssec,
            Algorithm::Dsa,
            "AQPJ////4Q==",
        );
        let ds_data = [DsDataType::new(
            12345,
            Algorithm::Dsa,
            DigestAlgorithm::Sha1,
            "49FD46E6C4B45C55D4AC",
            Some(key_data),
        )];
        let extension = CreateData::from((Duration::from_secs(604800), ds_data.as_ref()));
        let ns = [
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns1.example.com".into(),
            }),
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns2.example.com".into(),
            }),
        ];
        let contact = [
            domain::DomainContact {
                contact_type: "admin".into(),
                id: "sh8013".into(),
            },
            domain::DomainContact {
                contact_type: "tech".into(),
                id: "sh8013".into(),
            },
        ];
        let object = domain::DomainCreate::new(
            "example.com",
            Period::Years(PeriodLength::new(2).unwrap()),
            Some(&ns),
            Some("jd1234"),
            "2fooBAR",
            Some(&contact),
        );
        assert_serialized(
            "request/extensions/secdns_create_ds_key.xml",
            (&object, &extension),
        );
    }

    #[test]
    fn create_key_data_interface() {
        let key_data = [KeyDataType::new(
            FLAGS_DNS_ZONE_KEY_SEP,
            Protocol::Dnssec,
            Algorithm::RsaMd5,
            "AQPJ////4Q==",
        )];
        let extension = CreateData::from(key_data.as_ref());
        let ns = [
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns1.example.com".into(),
            }),
            domain::HostInfo::Obj(domain::HostObj {
                name: "ns2.example.com".into(),
            }),
        ];
        let contact = [
            domain::DomainContact {
                contact_type: "admin".into(),
                id: "sh8013".into(),
            },
            domain::DomainContact {
                contact_type: "tech".into(),
                id: "sh8013".into(),
            },
        ];
        let object = domain::DomainCreate::new(
            "example.com",
            Period::Years(PeriodLength::new(2).unwrap()),
            Some(&ns),
            Some("jd1234"),
            "2fooBAR",
            Some(&contact),
        );
        assert_serialized(
            "request/extensions/secdns_create_key.xml",
            (&object, &extension),
        );
    }
}
