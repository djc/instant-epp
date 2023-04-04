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

pub fn frnic_contact_create_physical_person(
    first_name: &str,
) -> Ext<Create<contact::FrnicContactCreatePp<'_>>> {
    Ext {
        data: Create {
            data: contact::FrnicContactCreatePp {
                first_name: first_name.into(),
            },
        },
    }
}

pub fn frnic_contact_create_company<'a>(
    siren: Option<&'a str>,
    vat: Option<&'a str>,
    trademark: Option<&'a str>,
    duns: Option<&'a str>,
    local: Option<&'a str>,
) -> Ext<Create<contact::FrnicContactCreateCorporation<'a>>> {
    Ext {
        data: Create {
            data: contact::FrnicContactCreateCorporation {
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

pub fn frnic_contact_create_non_profit<'a>(
    waldec: Option<&'a str>,
    decl: Option<&'a str>,
    publication: Option<contact::Publication<'a>>,
) -> Ext<Create<contact::FrnicContactCreateCorporation<'a>>> {
    Ext {
        data: Create {
            data: contact::FrnicContactCreateCorporation {
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
        contact::Publication, frnic_contact_create_company, frnic_contact_create_non_profit,
        frnic_contact_create_physical_person,
    };
    use crate::contact::create::ContactCreate;
    use crate::contact::{Address, PostalInfo, Voice};
    use crate::tests::assert_serialized;

    #[test]
    fn contact_create_physical_person() {
        // Guide d'intégration tecnhique, Septembre 2022, page 22.
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
            "Afn-12345678",
        );
        assert_serialized(
            "request/extensions/frnic_create_contact_physical_person.xml",
            (&object, &frnic_contact),
        );
    }

    #[test]
    fn contact_create_corporation() {
        // Guide d'intégration tecnhique, Septembre 2022, page 25.
        let frnic_contact = frnic_contact_create_company(None, None, None, None, None);
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
    fn contact_create_corporation_with_siren() {
        // Guide d'intégration tecnhique, Septembre 2022, page 26.
        let frnic_contact = frnic_contact_create_company(Some("123456789"), None, None, None, None);
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
    fn contact_create_non_profit() {
        // Guide d'intégration tecnhique, Septembre 2022, page 33.
        let frnic_contact = frnic_contact_create_non_profit(
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
