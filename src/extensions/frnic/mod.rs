use instant_xml::{FromXml, ToXml};

pub mod contact;

pub const XMLNS: &str = "http://www.afnic.fr/xml/epp/frnic-2.0";

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "ext", ns(XMLNS))]
pub struct Ext<T> {
    pub data: T,
}

#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "create", ns(XMLNS))]
pub struct Create<T> {
    pub data: T,
}

pub fn frnic_contact_create_physical_person<'a>(
    first_name: &'a str
) -> Ext<Create<contact::FrnicContactCreate<'a>>> {
    Ext {
        data: Create {
            data: contact::FrnicContactCreate {
                first_name: first_name.into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::frnic_contact_create_physical_person;
    use crate::contact::{Address, PostalInfo, Voice};
    use crate::contact::create::ContactCreate;
    use crate::tests::assert_serialized;

    #[test]
    fn contact_create_physical_person() {
        let frnic_contact = frnic_contact_create_physical_person("Michel");
        let object = ContactCreate::new(
            "XXX000",
            "test@test.fr",
            PostalInfo::new(
                "loc",
                "Dupont",
                None,
                Address::new(
                    &["1 Rue des fleurs"],
                    "Paris",
                    None,
                    Some("75000"),
                    "FR".parse().unwrap(),
                ),
            ),
            Some(Voice::new("+33.1234567890")),
            "Afn-12345678"
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_physical_person.xml",
            (&object, &frnic_contact),
        );
    }
}
