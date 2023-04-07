//! Mapping for the [frnic-2.0
//! extension](https://www.afnic.fr/wp-media/uploads/2022/10/guide_d_integration_technique_EN_FRandoverseasTLD_30_09_VF.pdf)
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

pub fn contact_create_physical_person(
    first_name: &str,
) -> Ext<Create<contact::ContactCreatePp<'_>>> {
    Ext {
        data: Create {
            data: contact::ContactCreatePp {
                first_name: first_name.into(),
            },
        },
    }
}

pub fn contact_create_company<'a>(
    siren: Option<&'a str>,
    vat: Option<&'a str>,
    trademark: Option<&'a str>,
    duns: Option<&'a str>,
    local: Option<&'a str>,
) -> Ext<Create<contact::ContactCreateCorporation<'a>>> {
    Ext {
        data: Create {
            data: contact::ContactCreateCorporation {
                legal_entity: contact::LegalEntityInfos {
                    legal_status: contact::LegalStatus::Company,
                    siren: siren.map(|s| s.into()),
                    vat: vat.map(|v| v.into()),
                    trademark: trademark.map(|t| t.into()),
                    asso: None,
                    duns: duns.map(|d| d.into()),
                    local: local.map(|l| l.into()),
                },
            },
        },
    }
}

pub fn contact_create_non_profit<'a>(
    waldec: Option<&'a str>,
    decl: Option<&'a str>,
    publication: Option<contact::Publication<'a>>,
) -> Ext<Create<contact::ContactCreateCorporation<'a>>> {
    Ext {
        data: Create {
            data: contact::ContactCreateCorporation {
                legal_entity: contact::LegalEntityInfos {
                    legal_status: contact::LegalStatus::NonProfit,
                    siren: None,
                    vat: None,
                    trademark: None,
                    asso: Some(contact::Asso {
                        waldec: waldec.map(|w| w.into()),
                        decl: decl.map(|d| d.into()),
                        publ: publication,
                    }),
                    duns: None,
                    local: None,
                },
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        contact::Publication, contact_create_company, contact_create_non_profit,
        contact_create_physical_person,
    };
    use crate::contact::create::ContactCreate;
    use crate::contact::{Address, PostalInfo, Voice};
    use crate::tests::assert_serialized;

    #[test]
    fn test_contact_create_physical_person() {
        // Technical Integration Guide, page 23.
        let frnic_contact = contact_create_physical_person("Michel");
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
            "request/extensions/frnic_create_contact_physical_person.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn test_contact_create_corporation() {
        // Technical Integration Guide, page 27.
        let frnic_contact = contact_create_company(None, None, None, None, None);
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
            "request/extensions/frnic_create_contact_corporation.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn test_contact_create_corporation_with_siren() {
        // Technical Integration Guide, page 28.
        let frnic_contact = contact_create_company(Some("123456789"), None, None, None, None);
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
        let frnic_contact = contact_create_non_profit(
            None,
            Some("2011-05-02"),
            Some(Publication {
                announce: 123456,
                page: 15,
                date: "2011-05-07".into(),
            }),
        );
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
