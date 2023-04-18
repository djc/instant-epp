//! DNS security extensions mapping
//!
//! As described in [RFC 5910](https://www.rfc-editor.org/rfc/rfc5910)
use instant_xml::{Accumulate, Error, Id, ToXml};
use std::borrow::Cow;
use std::time::Duration;

use crate::request::{Extension, Transaction};

pub const XMLNS: &str = "urn:ietf:params:xml:ns:secDNS-1.1";

impl<'a> Transaction<CreateData<'a>> for crate::domain::create::DomainCreate<'a> {}

impl<'a> Extension for CreateData<'a> {
    type Response = ();
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
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _: Option<Id<'_>>,
        serializer: &mut instant_xml::Serializer<'_, W>,
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

#[derive(Default)]
pub enum DsOrKeyTypeBuilder<'a> {
    #[default]
    None,
    MaxSigLife(Duration),
    Finished(Option<Duration>, DsOrKeyData<'a>),
}

impl<'a> Accumulate<DsOrKeyType<'a>> for DsOrKeyTypeBuilder<'a> {
    fn try_done(self, _: &'static str) -> Result<DsOrKeyType<'a>, Error> {
        if let Self::Finished(maximum_signature_lifetime, data) = self {
            Ok(DsOrKeyType {
                maximum_signature_lifetime,
                data,
            })
        } else {
            Err(Error::MissingTag)
        }
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
    digest_type: u8,
    digest: Cow<'a, str>,
    #[xml(rename = "keyData")]
    key_data: Option<KeyDataType<'a>>,
}

impl<'a> DsDataType<'a> {
    pub fn new(
        key_tag: u16,
        algorithm: Algorithm,
        digest_type: u8,
        digest: &'a str,
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

/// Algorithm identifies the public key's cryptographic algorithm
/// https://www.iana.org/assignments/dns-sec-alg-numbers/dns-sec-alg-numbers.xhtml#dns-sec-alg-numbers-1
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
    RSASHA1,
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
    ECDSAP384Sha384,
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
    Other(u8)
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
            Algorithm::RSASHA1 => 5,
            Algorithm::DsaNsec3Sha1 => 6,
            Algorithm::RsaSha1Nsec3Sha1 => 7,
            Algorithm::RsaSha256 => 8,
            Algorithm::RsaSha512 => 10,
            Algorithm::EccGost => 12,
            Algorithm::EcdsaP256Sha256 => 13,
            Algorithm::ECDSAP384Sha384 => 14,
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
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        id: Option<Id<'_>>,
        serializer: &mut instant_xml::Serializer<'_, W>,
    ) -> Result<(), Error> {
        let alg = u8::from(*self);
        if let Some(id) = id {
            let prefix = serializer.write_start(id.name, id.ns)?;
            serializer.end_start()?;
            alg.serialize(None, serializer)?;
            serializer.write_close(prefix, id.name)
        } else {
            alg.serialize(None, serializer)
        }
    }
}

#[derive(Debug, ToXml)]
#[xml(rename = "keyData", ns(XMLNS))]
pub struct KeyDataType<'a> {
    flags: u16,
    protocol: u8,
    #[xml(rename = "alg")]
    algorithm: Algorithm,
    #[xml(rename = "pubKey")]
    public_key: Cow<'a, str>,
}

impl<'a> KeyDataType<'a> {
    pub fn new(flags: u16, protocol: u8, algorithm: Algorithm, public_key: &'a str) -> Self {
        Self {
            flags,
            protocol,
            algorithm,
            public_key: public_key.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain;
    use crate::extensions::secdns;
    use crate::tests::assert_serialized;
    use std::time::Duration;

    #[test]
    fn create_ds_data_interface() {
        let ds_data = [secdns::DsDataType::new(
            12345,
            secdns::Algorithm::Dsa,
            1,
            "49FD46E6C4B45C55D4AC",
            None,
        )];
        let extension = secdns::CreateData::from((Duration::from_secs(604800), ds_data.as_ref()));
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
            domain::Period::years(2).unwrap(),
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
        let key_data = secdns::KeyDataType::new(257, 3, secdns::Algorithm::Dsa, "AQPJ////4Q==");
        let ds_data = [secdns::DsDataType::new(
            12345,
            secdns::Algorithm::Dsa,
            1,
            "49FD46E6C4B45C55D4AC",
            Some(key_data),
        )];
        let extension = secdns::CreateData::from((Duration::from_secs(604800), ds_data.as_ref()));
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
            domain::Period::years(2).unwrap(),
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
        let key_data = [secdns::KeyDataType::new(257, 3, secdns::Algorithm::RsaMd5, "AQPJ////4Q==")];
        let extension = secdns::CreateData::from(key_data.as_ref());
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
            domain::Period::years(2).unwrap(),
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
