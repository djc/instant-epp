//! Types for EPP FRNIC contact requests
use std::borrow::Cow;
use instant_xml::{FromXml, ToXml};

use crate::contact::create::ContactCreate;
use crate::request::{Extension, Transaction};

use super::{Ext, Create, XMLNS};

impl<'a> Transaction<Ext<Create<FrnicContactCreate<'a>>>> for ContactCreate<'a> {}

impl <'a> Extension for Ext<Create<FrnicContactCreate<'a>>> {
    type Response = ();
}

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "contact", ns(XMLNS))]
pub struct FrnicContactCreate<'a> {
    #[xml(rename = "firstName")]
    pub first_name: Cow<'a, str>
}
