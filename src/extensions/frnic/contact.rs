//! Types for EPP FRNIC contact requests
use instant_xml::{Id, ToXml};
use std::borrow::Cow;

use crate::request::{Extension, Transaction};

use super::{Create, Ext, XMLNS};

impl<'a> Transaction<Ext<Create<ContactCreate<'a>>>> for crate::contact::create::ContactCreate<'a> {}

impl Extension for Ext<Create<ContactCreate<'_>>> {
    type Response = ();
}

/// For french TLDs, a contact is either an individual (PP) or a legal
/// entity (PM). We use the `ContactCreate` extension to differentiate
/// between the creation of a PP and a PM.
#[derive(Debug)]
pub enum ContactCreate<'a> {
    /// This contact is an individual.
    NaturalPerson {
        /// First name of the contact. The `<contact:name>` element
        /// will be the family name.
        first_name: Cow<'a, str>,
    },
    /// This contact is a legal entity.
    LegalEntity(Box<LegalEntityInfos<'a>>),
}

impl<'a> From<ContactCreate<'a>> for Ext<Create<ContactCreate<'a>>> {
    fn from(data: ContactCreate<'a>) -> Self {
        Ext {
            data: Create { data },
        }
    }
}

impl<'a> ContactCreate<'a> {
    pub fn new_natural_person(first_name: &'a str) -> Self {
        Self::NaturalPerson {
            first_name: first_name.into(),
        }
    }

    pub fn new_company(
        siren: Option<&'a str>,
        vat: Option<&'a str>,
        trademark: Option<&'a str>,
        duns: Option<&'a str>,
        local: Option<&'a str>,
    ) -> Self {
        Self::LegalEntity(Box::new(LegalEntityInfos {
            legal_status: LegalStatus::Company,
            siren: siren.map(|s| s.into()),
            vat: vat.map(|v| v.into()),
            trademark: trademark.map(|t| t.into()),
            asso: None,
            duns: duns.map(|d| d.into()),
            local: local.map(|l| l.into()),
        }))
    }

    pub fn new_non_profit(
        waldec: Option<&'a str>,
        declaration: Option<&'a str>,
        publication: Option<Publication<'a>>,
    ) -> Self {
        Self::LegalEntity(Box::new(LegalEntityInfos {
            legal_status: LegalStatus::Association,
            siren: None,
            vat: None,
            trademark: None,
            asso: Some(Association {
                waldec: waldec.map(|w| w.into()),
                declaration: declaration.map(|d| d.into()),
                publication,
            }),
            duns: None,
            local: None,
        }))
    }
}

impl ToXml for ContactCreate<'_> {
    fn serialize<W: core::fmt::Write + ?Sized>(
        &self,
        _: Option<Id<'_>>,
        serializer: &mut instant_xml::Serializer<'_, W>,
    ) -> Result<(), instant_xml::Error> {
        let contact_nc_name = "contact";
        let prefix = serializer.write_start(contact_nc_name, XMLNS)?;
        serializer.end_start()?;
        match self {
            Self::NaturalPerson { first_name } => {
                let first_name_nc_name = "firstName";
                let prefix = serializer.write_start(first_name_nc_name, XMLNS)?;
                serializer.end_start()?;
                first_name.serialize(None, serializer)?;
                serializer.write_close(prefix, first_name_nc_name)?;
            }
            Self::LegalEntity(infos) => infos.serialize(None, serializer)?,
        }
        serializer.write_close(prefix, contact_nc_name)?;
        Ok(())
    }
}

#[derive(Debug, ToXml)]
#[xml(rename = "legalEntityInfos", ns(XMLNS))]
pub struct LegalEntityInfos<'a> {
    pub legal_status: LegalStatus<'a>,
    pub siren: Option<Cow<'a, str>>,
    pub vat: Option<Cow<'a, str>>,
    pub trademark: Option<Cow<'a, str>>,
    pub asso: Option<Association<'a>>,
    pub duns: Option<Cow<'a, str>>,
    pub local: Option<Cow<'a, str>>,
}

#[derive(Debug)]
pub enum LegalStatus<'a> {
    Company,
    Association,
    Other(Cow<'a, str>),
}

impl ToXml for LegalStatus<'_> {
    fn serialize<W: core::fmt::Write + ?Sized>(
        &self,
        _field: Option<Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let ncname = "legalStatus";
        let (s, data) = match self {
            LegalStatus::Company => ("company", None),
            LegalStatus::Association => ("association", None),
            LegalStatus::Other(text) => ("other", Some(&text.as_ref()[2..])),
        };
        let prefix = serializer.write_start(ncname, XMLNS)?;
        debug_assert_eq!(prefix, None);
        serializer.write_attr("s", XMLNS, s)?;
        if let Some(text) = data {
            serializer.end_start()?;
            text.serialize(None, serializer)?;
            serializer.write_close(prefix, ncname)?;
        } else {
            serializer.end_empty()?;
        }
        Ok(())
    }
}

/// Contains information that permits the identification of associations.
#[derive(Debug, ToXml)]
#[xml(rename = "asso", ns(XMLNS))]
pub struct Association<'a> {
    /// The Waldec registration number. "Waldec" is the acronym for
    /// the french "[Web des associations librement
    /// déclarées](https://www.associations.gouv.fr/le-rna-repertoire-national-des-associations.html)"
    pub waldec: Option<Cow<'a, str>>,
    /// Date of declaration to the prefecture
    #[xml(rename = "decl")]
    pub declaration: Option<Cow<'a, str>>,
    /// Information of publication in the official gazette
    #[xml(rename = "publ")]
    pub publication: Option<Publication<'a>>,
}

/// Holds information about the publication in the
/// official gazette for the association.
#[derive(Debug, ToXml)]
#[xml(rename = "publ", ns(XMLNS))]
pub struct Publication<'a> {
    /// Page number of the announcement
    #[xml(attribute)]
    pub page: u32,
    #[xml(attribute)]
    /// Number of the announcement
    pub announce: u32,
    /// Date of publication
    #[xml(direct)]
    pub date: Cow<'a, str>,
}
