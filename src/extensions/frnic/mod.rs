//! Mapping for the [frnic-2.0
//! extension](https://www.afnic.fr/wp-media/uploads/2022/10/guide_d_integration_technique_EN_FRandoverseasTLD_30_09_VF.pdf)
use instant_xml::{FromXml, ToXml};

pub mod contact;

pub use contact::ContactCreate;

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

#[cfg(test)]
mod tests {
    use crate::contact::ContactCreate;
    use crate::contact::{Address, PostalInfo, Voice};
    use crate::extensions::frnic;
    use crate::tests::assert_serialized;
    use frnic::{contact, Ext};

    #[test]
    fn test_contact_create_natural_person() {
        // Technical Integration Guide, page 23.
        let frnic_contact = Ext::from(frnic::ContactCreate::new_natural_person("Michel"));
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
            "Afn-12345678",
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_natural_person.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn test_contact_create_company() {
        // Technical Integration Guide, page 27.
        let frnic_contact = Ext::from(frnic::ContactCreate::new_company(
            None, None, None, None, None,
        ));
        let object = ContactCreate::new(
            "XXXXXXX",
            "test@test.fr",
            PostalInfo::new(
                "loc",
                "SARL DUPONT",
                None,
                Address::new(
                    &["1 Rue des coquelicots"],
                    "Paris",
                    None,
                    Some("75000"),
                    "FR".parse().unwrap(),
                ),
            ),
            Some(Voice::new("+33.1234567890")),
            "Afn-123456",
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_company.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn test_contact_create_corporation_with_siren() {
        // Technical Integration Guide, page 28.
        let frnic_contact = Ext::from(frnic::ContactCreate::new_company(
            Some("123456789"),
            None,
            None,
            None,
            None,
        ));
        let object = ContactCreate::new(
            "XXXX0000",
            "test@test.fr",
            PostalInfo::new(
                "loc",
                "SARL DUPONT SIREN",
                None,
                Address::new(
                    &["1 Rue des Sirenes"],
                    "Paris",
                    None,
                    Some("75000"),
                    "FR".parse().unwrap(),
                ),
            ),
            Some(Voice::new("+33.1234567890")),
            "Afn-123456",
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_siren.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn test_contact_create_non_profit() {
        // Technical Integration Guide, page 38.
        let frnic_contact = Ext::from(frnic::ContactCreate::new_non_profit(
            None,
            Some("2011-05-02"),
            Some(contact::Publication {
                announce: 123456,
                page: 15,
                date: "2011-05-07".into(),
            }),
        ));
        let object = ContactCreate::new(
            "XXXX0000",
            "test@test.fr",
            PostalInfo::new(
                "loc",
                "Dupont JO",
                None,
                Address::new(
                    &["1 Rue des Fleurs"],
                    "Paris",
                    None,
                    Some("75000"),
                    "FR".parse().unwrap(),
                ),
            ),
            Some(Voice::new("+33.1234567890")),
            "Afn-123456",
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_non_profit.xml",
            (&object, &frnic_contact),
        );
    }
}
