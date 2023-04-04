//! Types for EPP FRNIC contact requests
use instant_xml::{FromXml, Id, ToXml};
use std::borrow::Cow;

use crate::contact::create::ContactCreate;
use crate::request::{Extension, Transaction};

use super::{Create, Ext, XMLNS};

impl<'a> Transaction<Ext<Create<FrnicContactCreatePp<'a>>>> for ContactCreate<'a> {}

impl<'a> Extension for Ext<Create<FrnicContactCreatePp<'a>>> {
    type Response = ();
}

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "contact", ns(XMLNS))]
pub struct FrnicContactCreatePp<'a> {
    #[xml(rename = "firstName")]
    pub first_name: Cow<'a, str>,
}

impl<'a> Transaction<Ext<Create<FrnicContactCreateCorporation<'a>>>> for ContactCreate<'a> {}

impl<'a> Extension for Ext<Create<FrnicContactCreateCorporation<'a>>> {
    type Response = ();
}

#[derive(Debug, ToXml)]
#[xml(rename = "contact", ns(XMLNS))]
pub struct FrnicContactCreateCorporation<'a> {
    pub legal_entity: LegalEntityInfos<'a>,
}

#[derive(Debug, ToXml)]
#[xml(rename = "legalEntityInfos", ns(XMLNS))]
pub struct LegalEntityInfos<'a> {
    #[xml(rename = "legalStatus")]
    pub legal_status: LegalStatus<'a>,
    pub siren: Option<Cow<'a, str>>,
    pub vat: Option<Cow<'a, str>>,
    pub trademark: Option<Cow<'a, str>>,
    pub asso: Option<Asso<'a>>,
    pub duns: Option<Cow<'a, str>>,
    pub local: Option<Cow<'a, str>>,
}

#[derive(Debug)]
pub enum LegalStatus<'a> {
    Company,
    NonProfit,
    Other(Cow<'a, str>),
}

impl<'a> ToXml for LegalStatus<'a> {
    fn serialize<W: core::fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let field = field.expect("field for LegalStatus has a name");
        let (s, data) = match self {
            LegalStatus::Company => ("company", ""),
            LegalStatus::NonProfit => ("association", ""),
            LegalStatus::Other(data) => ("other", data.as_ref()),
        };
        let prefix = serializer.write_start(field.name, field.ns)?;
        debug_assert_eq!(prefix, None);
        serializer.write_attr("s", XMLNS, s)?;
        if data.is_empty() {
            serializer.end_empty()?;
        } else {
            serializer.end_start()?;
            data.serialize(None, serializer)?;
            serializer.write_close(prefix, field.name)?;
        }
        Ok(())
    }
}

#[derive(Debug, ToXml)]
#[xml(rename = "asso", ns(XMLNS))]
pub struct Asso<'a> {
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
