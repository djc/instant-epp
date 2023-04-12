//! Types for EPP FRNIC contact requests
use instant_xml::{Id, ToXml};
use std::borrow::Cow;

use crate::contact::create::ContactCreate;
use crate::request::{Extension, Transaction};

use super::{Create, Ext, XMLNS};

impl<'a> Transaction<Ext<Create<CreateData<'a>>>> for ContactCreate<'a> {}

impl<'a> Extension for Ext<Create<CreateData<'a>>> {
    type Response = ();
}

#[derive(Debug)]
pub enum CreateData<'a> {
    NaturalPerson { first_name: Cow<'a, str> },
    LegalEntity(Box<LegalEntityInfos<'a>>),
}

impl<'a> ToXml for CreateData<'a> {
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

impl<'a> ToXml for LegalStatus<'a> {
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

#[derive(Debug, ToXml)]
#[xml(rename = "asso", ns(XMLNS))]
pub struct Association<'a> {
    pub waldec: Option<Cow<'a, str>>,
    pub decl: Option<Cow<'a, str>>,
    pub publ: Option<Publication<'a>>,
}

#[derive(Debug, ToXml)]
#[xml(rename = "publ", ns(XMLNS))]
pub struct Publication<'a> {
    #[xml(attribute)]
    pub page: u32,
    #[xml(attribute)]
    pub announce: u32,
    #[xml(direct)]
    pub date: Cow<'a, str>,
}
