//! Types for EPP FRNIC contact requests
use std::borrow::Cow;
use instant_xml::{FromXml, ToXml};

use crate::contact::create::ContactCreate;
use crate::request::{Extension, Transaction};

use super::{Ext, Create, XMLNS};

impl<'a> Transaction<Ext<Create<FrnicContactCreatePp<'a>>>> for ContactCreate<'a> {}

impl <'a> Extension for Ext<Create<FrnicContactCreatePp<'a>>> {
    type Response = ();
}

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "contact", ns(XMLNS))]
pub struct FrnicContactCreatePp<'a> {
    #[xml(rename = "firstName")]
    pub first_name: Cow<'a, str>
}
