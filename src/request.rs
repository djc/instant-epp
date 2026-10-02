//! Types for EPP requests

use std::fmt::Debug;

use instant_xml::ser::Context;
use instant_xml::{FromXmlOwned, ToXml};

use crate::common::EPP_XMLNS;

pub const EPP_VERSION: &str = "1.0";
pub const EPP_LANG: &str = "en";

/// Trait to set correct value for xml tags when tags are being generated from generic types
pub trait Transaction<Ext: Extension>: Command + Sized {}

pub trait Command: ToXml + Debug {
    type Response: FromXmlOwned + Debug;
    const COMMAND: &'static str;
}

pub trait Extension: ToXml + Debug {
    type Response: FromXmlOwned + Debug;
}

#[derive(Debug, PartialEq)]
/// Type corresponding to the `<command>` tag in an EPP XML request
/// with an `<extension>` tag
pub(crate) struct CommandWrapper<'a, D, E> {
    command: &'static str,
    /// The instance that will be used to populate the `<command>` tag
    data: &'a D,
    /// The client TRID
    extension: Option<&'a E>,
    client_tr_id: String,
}

impl<'a, E: Extension, D: Transaction<E>> CommandWrapper<'a, D, E> {
    pub(crate) fn new(data: &'a D, extension: Option<&'a E>, client_tr_id: &'a str) -> Self {
        Self {
            command: D::COMMAND,
            data,
            extension,
            client_tr_id: client_tr_id.into(),
        }
    }
}

impl<D: ToXml, E: ToXml> ToXml for CommandWrapper<'_, D, E> {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let command = serializer.write_start("command", EPP_XMLNS, None::<Context<0>>)?;
        serializer.end_start()?;
        self.data.serialize(None, serializer)?;
        if let Some(extension) = self.extension {
            Ext { inner: extension }.serialize(None, serializer)?;
        }

        let cl_tr_id = serializer.write_start("clTRID", EPP_XMLNS, None::<Context<0>>)?;
        serializer.end_start()?;
        self.client_tr_id.as_str().serialize(None, serializer)?;
        serializer.write_close(cl_tr_id)?;

        serializer.write_close(command)?;
        Ok(())
    }
}

#[derive(Debug, ToXml)]
#[xml(rename = "extension", ns(EPP_XMLNS))]
struct Ext<E> {
    inner: E,
}

#[cfg(test)]
mod tests {
    use similar_asserts::assert_eq;

    use super::CommandWrapper;
    use crate::common::NoExtension;
    use crate::domain::DomainCheck;
    use crate::tests::{get_xml, CLTRID};
    use crate::xml;

    #[test]
    fn client_transaction_id_is_escaped() {
        let object = DomainCheck {
            domains: &["eppdev.com", "eppdev.net"],
        };
        let document =
            CommandWrapper::new(&object, None::<&NoExtension>, "x</clTRID><foo/><clTRID>a&b");

        let expected = get_xml("request/domain/check.xml")
            .unwrap()
            .replace(CLTRID, "x&lt;/clTRID&gt;&lt;foo/&gt;&lt;clTRID&gt;a&amp;b");
        assert_eq!(expected, xml::serialize(document).unwrap());
    }
}
