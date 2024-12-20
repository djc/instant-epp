//! Types for EPP contact create request

use instant_xml::ToXml;

use super::{ContactAuthInfo, Fax, PostalInfo, Status, Voice, XMLNS};
use crate::common::{NoExtension, EPP_XMLNS};
use crate::request::{Command, Transaction};

impl Transaction<NoExtension> for ContactUpdate<'_> {}

impl Command for ContactUpdate<'_> {
    type Response = ();
    const COMMAND: &'static str = "update";
}

impl<'a> ContactUpdate<'a> {
    pub fn new(id: &'a str) -> Self {
        Self {
            contact: ContactUpdateRequest {
                id,
                add_statuses: None,
                remove_statuses: None,
                change_info: None,
            },
        }
    }

    /// Sets the data for the `<chg>` tag for the contact update request
    pub fn set_info(
        &mut self,
        email: &'a str,
        postal_info: PostalInfo<'a>,
        voice: Voice<'a>,
        auth_password: &'a str,
    ) {
        self.contact.change_info = Some(ContactChangeInfo {
            email: Some(email),
            postal_info: Some(postal_info),
            voice: Some(voice),
            auth_info: Some(ContactAuthInfo::new(auth_password)),
            fax: None,
        });
    }

    /// Sets the data for the `<fax>` tag under `<chg>` for the contact update request
    pub fn set_fax(&mut self, fax: Fax<'a>) {
        if let Some(info) = &mut self.contact.change_info {
            info.fax = Some(fax)
        }
    }

    /// Sets the data for the `<add>` tag for the contact update request
    pub fn add(&mut self, statuses: &'a [Status]) {
        self.contact.add_statuses = Some(AddStatuses { statuses });
    }

    /// Sets the data for the `<rem>` tag for the contact update request
    pub fn remove(&mut self, statuses: &'a [Status]) {
        self.contact.remove_statuses = Some(RemoveStatuses { statuses });
    }
}

/// Type for elements under the `<chg>` tag for contact update request
#[derive(Debug, ToXml)]
#[xml(rename = "chg", ns(XMLNS))]
pub struct ContactChangeInfo<'a> {
    postal_info: Option<PostalInfo<'a>>,
    voice: Option<Voice<'a>>,
    fax: Option<Fax<'a>>,
    email: Option<&'a str>,
    auth_info: Option<ContactAuthInfo<'a>>,
}

#[derive(Debug, ToXml)]
#[xml(rename = "add", ns(XMLNS))]
struct AddStatuses<'a> {
    statuses: &'a [Status],
}

#[derive(Debug, ToXml)]
#[xml(rename = "rem", ns(XMLNS))]
struct RemoveStatuses<'a> {
    statuses: &'a [Status],
}

/// Type for elements under the contact `<update>` tag
#[derive(Debug, ToXml)]
#[xml(rename = "update", ns(XMLNS))]
pub struct ContactUpdateRequest<'a> {
    id: &'a str,
    add_statuses: Option<AddStatuses<'a>>,
    #[xml(rename = "rem")]
    remove_statuses: Option<RemoveStatuses<'a>>,
    change_info: Option<ContactChangeInfo<'a>>,
}

/// Type for EPP XML `<update>` command for contacts
#[derive(Debug, ToXml)]
#[xml(rename = "update", ns(EPP_XMLNS))]
pub struct ContactUpdate<'a> {
    /// The data under the `<update>` tag for the contact update
    contact: ContactUpdateRequest<'a>,
}

#[cfg(test)]
mod tests {
    use super::{ContactUpdate, PostalInfo, Status, Voice};
    use crate::contact::{Address, InfoType};
    use crate::response::ResultCode;
    use crate::tests::{assert_serialized, response_from_file, CLTRID, SUCCESS_MSG, SVTRID};

    #[test]
    fn command() {
        let mut object = ContactUpdate::new("eppdev-contact-3");

        let street = &["58", "Orchid Road"];
        let address = Address::new(
            street,
            "Paris",
            Some("Paris"),
            Some("392374"),
            "FR".parse().unwrap(),
        );
        let postal_info =
            PostalInfo::new(InfoType::Local, "John Doe", Some("Acme Widgets"), address);
        let voice = Voice::new("+33.47237942");

        object.set_info("newemail@eppdev.net", postal_info, voice, "eppdev-387323");
        object.add(&[Status::ClientTransferProhibited]);
        object.remove(&[Status::ClientDeleteProhibited]);

        assert_serialized("request/contact/update.xml", &object);
    }

    #[test]
    fn contact_update() {
        let object = response_from_file::<ContactUpdate>("response/contact/update.xml");
        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);
        assert_eq!(object.result.message, SUCCESS_MSG);
        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
