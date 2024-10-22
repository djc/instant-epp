//! Types to use in serialization to and deserialization from EPP XML

use instant_xml::{FromXml, FromXmlOwned, ToXml};

use crate::common::EPP_XMLNS;
use crate::error::Error;

pub const EPP_XML_HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#;

pub(crate) fn serialize(data: impl ToXml) -> Result<String, Error> {
    Ok(format!(
        "{}\r\n{}",
        EPP_XML_HEADER,
        instant_xml::to_string(&Epp { data }).map_err(|e| Error::Xml(e.into()))?
    ))
}

pub(crate) fn deserialize<T: FromXmlOwned>(xml: &str) -> Result<T, Error> {
    match instant_xml::from_str::<Epp<T>>(xml) {
        Ok(Epp { data }) => Ok(data),
        Err(e) => Err(Error::Xml(e.into())),
    }
}

#[derive(FromXml, ToXml)]
#[xml(rename = "epp", ns(EPP_XMLNS))]
pub(crate) struct Epp<T> {
    pub(crate) data: T,
}

macro_rules! from_scalar {
    ($name:ty, $scalar:tt) => {
        impl<'xml> ::instant_xml::FromXml<'xml> for $name {
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                match field {
                    Some(field) => id == field,
                    None => false,
                }
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                let mut value = None;

                $scalar::deserialize(&mut value, field, deserializer)?;

                if let Some(value) = value {
                    *into = Some(Self::from(value));
                }

                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Scalar;
        }
    };
}
pub(crate) use from_scalar;
