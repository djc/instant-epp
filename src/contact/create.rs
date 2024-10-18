//! Types for EPP contact create request

use chrono::{DateTime, Utc};
use instant_xml::{FromXml, ToXml};

use super::{ContactAuthInfo, Fax, PostalInfo, Voice, XMLNS};
use crate::common::{NoExtension, EPP_XMLNS};
use crate::request::{Command, Transaction};

impl<'a> Transaction<NoExtension> for ContactCreate<'a> {}

impl<'a> Command for ContactCreate<'a> {
    type Response = CreateData;
    const COMMAND: &'static str = "create";
}

// Request

/// Type for elements under the contact `<create>` tag
#[derive(Debug, ToXml)]
#[xml(rename = "create", ns(XMLNS))]
pub struct ContactCreateRequest<'a> {
    /// Contact `<id>` tag
    id: &'a str,
    /// Contact `<postalInfo>` tag
    postal_info: PostalInfo<'a>,
    /// Contact `<voice>` tag
    voice: Option<Voice<'a>>,
    /// Contact `<fax>` tag,]
    fax: Option<Fax<'a>>,
    /// Contact `<email>` tag
    email: &'a str,
    /// Contact `<authInfo>` tag
    auth_info: ContactAuthInfo<'a>,
}

/// Type for EPP XML `<create>` command for contacts
#[derive(Debug, ToXml)]
#[xml(rename = "create", ns(EPP_XMLNS))]
pub struct ContactCreate<'a> {
    /// Data for `<create>` command for contact
    pub contact: ContactCreateRequest<'a>,
}

impl<'a> ContactCreate<'a> {
    pub fn new(
        id: &'a str,
        email: &'a str,
        postal_info: PostalInfo<'a>,
        voice: Option<Voice<'a>>,
        auth_password: &'a str,
    ) -> Self {
        Self {
            contact: ContactCreateRequest {
                id,
                postal_info,
                voice,
                fax: None,
                email,
                auth_info: ContactAuthInfo::new(auth_password),
            },
        }
    }

    /// Sets the `<fax>` data for the request
    pub fn set_fax(&mut self, fax: Fax<'a>) {
        self.contact.fax = Some(fax);
    }
}

// Response

/// Type that represents the `<creData>` tag for contact create response
#[derive(Debug, FromXml)]
#[xml(rename = "creData", ns(XMLNS))]
pub struct CreateData {
    /// The contact id
    pub id: String,
    #[xml(rename = "crDate")]
    /// The contact creation date
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{ContactCreate, Fax, PostalInfo, Voice};
    use crate::contact::{Address, InfoType};
    use crate::response::ResultCode;
    use crate::tests::{assert_serialized, response_from_file, CLTRID, SUCCESS_MSG, SVTRID};

    #[test]
    fn command() {
        let street = &["58", "Orchid Road"];
        let address = Address::new(
            street,
            "Paris",
            Some("Paris"),
            Some("392374"),
            "FR".parse().unwrap(),
        );
        let postal_info = PostalInfo::new(
            InfoType::International,
            "John Doe",
            Some("Acme Widgets"),
            address,
        );
        let mut voice = Voice::new("+33.47237942");
        voice.set_extension("123");
        let mut fax = Fax::new("+33.86698799");
        fax.set_extension("677");

        let mut object = ContactCreate::new(
            "eppdev-contact-3",
            "contact@eppdev.net",
            postal_info,
            Some(voice),
            "eppdev-387323",
        );
        object.set_fax(fax);

        assert_serialized("request/contact/create.xml", &object);
    }

    #[test]
    fn command_minimal() {
        let address = Address::new(&[], "Paris", None, None, "FR".parse().unwrap());
        let postal_info = PostalInfo::new(InfoType::International, "John Doe", None, address);
        let object = ContactCreate::new(
            "eppdev-contact-3",
            "contact@eppdev.net",
            postal_info,
            None,
            "eppdev-387323",
        );

        assert_serialized("request/contact/create_minimal.xml", &object);
    }

    #[test]
    fn response() {
        let object = response_from_file::<ContactCreate>("response/contact/create.xml");
        let results = object.res_data().unwrap();

        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);
        assert_eq!(object.result.message, SUCCESS_MSG);
        assert_eq!(results.id, "eppdev-contact-4");
        assert_eq!(
            results.created_at,
            Utc.with_ymd_and_hms(2021, 7, 25, 16, 5, 32).unwrap(),
        );
        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
