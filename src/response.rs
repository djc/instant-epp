//! Types for EPP responses

use std::fmt::Debug;

use chrono::{DateTime, Utc};
use instant_xml::{FromXml, Kind};

use crate::common::EPP_XMLNS;

/// A dynamically captured XML element from an EPP `<value>` tag.
///
/// Satifies:
/// ```xml
/// <any namespace="##any" processContents="skip" />
/// ```
///
/// This type captures an element tree dynamically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueElement {
    /// XML namespace URI of this element
    pub ns: String,
    /// Local element name
    pub name: String,
    /// Attributes as (prefix, name, value) pairs
    pub attributes: Vec<(Option<String>, String, String)>,
    /// Mapping of prefix to namespace URI declared on this element
    pub namespaces: Vec<(String, String)>,
    /// Text content, if any
    pub text: Option<String>,
    /// Nested child elements
    pub children: Vec<Self>,
}

impl ValueElement {
    fn deserialize_element(
        deserializer: &mut instant_xml::Deserializer<'_, '_>,
        id: instant_xml::Id<'_>,
    ) -> Result<Self, instant_xml::Error> {
        use instant_xml::de::Node;

        let mut elem = Self {
            ns: id.ns.to_string(),
            name: id.name.to_string(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
            namespaces: Vec::new(),
        };

        loop {
            match deserializer.next() {
                Some(Ok(Node::Attribute(attr))) => {
                    // Capture namespace declarations as well, since these are relevant for interpreting the element
                    if attr.prefix == Some("xmlns") {
                        elem.namespaces
                            .push((attr.local.to_string(), attr.value.to_string()));
                    } else {
                        elem.attributes.push((
                            attr.prefix.map(|p| p.to_owned()),
                            attr.local.to_owned(),
                            attr.value.into_owned(),
                        ));
                    }
                }
                Some(Ok(Node::Open(element))) => {
                    let child_id = deserializer.element_id(&element)?;
                    let mut nested = deserializer.nested(element);
                    elem.children
                        .push(Self::deserialize_element(&mut nested, child_id)?);
                }
                Some(Ok(Node::Text(text))) => {
                    elem.text = Some(text.to_string());
                }
                Some(Ok(Node::Close { .. })) => break,
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }

        Ok(elem)
    }
}

/// Type corresponding to the `<value>` tag (errValueType) in an EPP response XML.
///
/// Contains arbitrary XML children.
/// Per RFC 5730, `errValueType` is
///
/// ```xml
/// <complexType name="errValueType" mixed="true">
///   <sequence>
///     <any namespace="##any" processContents="skip" />
///   </sequence>
///   <anyAttribute namespace="##any" processContents="skip" />
/// </complexType>
/// ```
/// This means that the `<value>` element can contain arbitrary nested XML elements
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultValue {
    /// Attributes on the `<value>` element itself (from `anyAttribute`)
    pub attributes: Vec<(Option<String>, String, String)>,
    /// The captured the inner element
    pub inner: ValueElement,
}

impl<'xml> FromXml<'xml> for ResultValue {
    fn matches(id: instant_xml::Id<'_>, field: Option<instant_xml::Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => {
                id == instant_xml::Id {
                    ns: EPP_XMLNS,
                    name: "value",
                }
            }
        }
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        _field: &'static str,
        deserializer: &mut instant_xml::Deserializer<'cx, 'xml>,
    ) -> Result<(), instant_xml::Error> {
        use instant_xml::de::Node;

        let mut attributes = Vec::new();
        let mut inner = None;
        loop {
            match deserializer.next() {
                Some(Ok(Node::Attribute(attr))) => {
                    attributes.push((
                        attr.prefix.map(|p| p.to_owned()),
                        attr.local.to_owned(),
                        attr.value.into_owned(),
                    ));
                }
                Some(Ok(Node::Open(element))) => {
                    let id = deserializer.element_id(&element)?;
                    let mut nested = deserializer.nested(element);
                    inner = Some(ValueElement::deserialize_element(&mut nested, id)?);
                }
                Some(Ok(Node::Close { .. })) => break,
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }

        *into = Some(Self {
            attributes,
            inner: inner.unwrap(),
        });
        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: Kind = Kind::Element;
}

/// Type corresponding to the `<extValue>` tag in an EPP response XML
#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "extValue", ns(EPP_XMLNS))]
pub struct ExtValue {
    /// Data under the `<value>` tag
    pub value: ResultValue,
    /// Data under the `<reason>` tag
    pub reason: Reason,
}

/// Type corresponding to the `<reason>` tag in an EPP `<extValue>` response XML
///
/// Per RFC 5730, the language is identified via an optional "lang" attribute.
/// If not specified, the default value is "en" (English).
#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "reason", ns(EPP_XMLNS))]
pub struct Reason {
    /// Language of the reason message (defaults to "en" if absent)
    #[xml(attribute)]
    pub lang: Option<String>,
    /// The human-readable reason text
    #[xml(direct)]
    pub text: String,
}

/// Type corresponding to the `<result>` tag in an EPP response XML
///
/// Per RFC 5730, a result can contain zero or more `<value>` and `<extValue>`
/// elements in any order.
#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "result", ns(EPP_XMLNS))]
pub struct EppResult {
    /// The result code
    #[xml(attribute)]
    pub code: ResultCode,
    /// The result message
    #[xml(rename = "msg")]
    pub message: String,
    /// Data under `<value>` tags
    #[xml(rename = "value")]
    pub values: Vec<ResultValue>,
    /// Data under `<extValue>` tags
    #[xml(rename = "extValue")]
    pub ext_values: Vec<ExtValue>,
}

/// Response codes as enumerated in section 3 of RFC 5730
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultCode {
    CommandCompletedSuccessfully = 1000,
    CommandCompletedSuccessfullyActionPending = 1001,
    CommandCompletedSuccessfullyNoMessages = 1300,
    CommandCompletedSuccessfullyAckToDequeue = 1301,
    CommandCompletedSuccessfullyEndingSession = 1500,
    UnknownCommand = 2000,
    CommandSyntaxError = 2001,
    CommandUseError = 2002,
    RequiredParameterMissing = 2003,
    ParameterValueRangeError = 2004,
    ParameterValueSyntaxError = 2005,
    UnimplementedProtocolVersion = 2100,
    UnimplementedCommand = 2101,
    UnimplementedOption = 2102,
    UnimplementedExtension = 2103,
    BillingFailure = 2104,
    ObjectIsNotEligibleForRenewal = 2105,
    ObjectIsNotEligibleForTransfer = 2106,
    AuthenticationError = 2200,
    AuthorizationError = 2201,
    InvalidAuthorizationInformation = 2202,
    ObjectPendingTransfer = 2300,
    ObjectNotPendingTransfer = 2301,
    ObjectExists = 2302,
    ObjectDoesNotExist = 2303,
    ObjectStatusProhibitsOperation = 2304,
    ObjectAssociationProhibitsOperation = 2305,
    ParameterValuePolicyError = 2306,
    UnimplementedObjectService = 2307,
    DataManagementPolicyViolation = 2308,
    CommandFailed = 2400,
    CommandFailedServerClosingConnection = 2500,
    AuthenticationErrorServerClosingConnection = 2501,
    SessionLimitExceededServerClosingConnection = 2502,
}

impl ResultCode {
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            1000 => Some(Self::CommandCompletedSuccessfully),
            1001 => Some(Self::CommandCompletedSuccessfullyActionPending),
            1300 => Some(Self::CommandCompletedSuccessfullyNoMessages),
            1301 => Some(Self::CommandCompletedSuccessfullyAckToDequeue),
            1500 => Some(Self::CommandCompletedSuccessfullyEndingSession),
            2000 => Some(Self::UnknownCommand),
            2001 => Some(Self::CommandSyntaxError),
            2002 => Some(Self::CommandUseError),
            2003 => Some(Self::RequiredParameterMissing),
            2004 => Some(Self::ParameterValueRangeError),
            2005 => Some(Self::ParameterValueSyntaxError),
            2100 => Some(Self::UnimplementedProtocolVersion),
            2101 => Some(Self::UnimplementedCommand),
            2102 => Some(Self::UnimplementedOption),
            2103 => Some(Self::UnimplementedExtension),
            2104 => Some(Self::BillingFailure),
            2105 => Some(Self::ObjectIsNotEligibleForRenewal),
            2106 => Some(Self::ObjectIsNotEligibleForTransfer),
            2200 => Some(Self::AuthenticationError),
            2201 => Some(Self::AuthorizationError),
            2202 => Some(Self::InvalidAuthorizationInformation),
            2300 => Some(Self::ObjectPendingTransfer),
            2301 => Some(Self::ObjectNotPendingTransfer),
            2302 => Some(Self::ObjectExists),
            2303 => Some(Self::ObjectDoesNotExist),
            2304 => Some(Self::ObjectStatusProhibitsOperation),
            2305 => Some(Self::ObjectAssociationProhibitsOperation),
            2306 => Some(Self::ParameterValuePolicyError),
            2307 => Some(Self::UnimplementedObjectService),
            2308 => Some(Self::DataManagementPolicyViolation),
            2400 => Some(Self::CommandFailed),
            2500 => Some(Self::CommandFailedServerClosingConnection),
            2501 => Some(Self::AuthenticationErrorServerClosingConnection),
            2502 => Some(Self::SessionLimitExceededServerClosingConnection),
            _ => None,
        }
    }

    pub fn is_success(&self) -> bool {
        use ResultCode::*;
        matches!(
            self,
            CommandCompletedSuccessfully
                | CommandCompletedSuccessfullyActionPending
                | CommandCompletedSuccessfullyNoMessages
                | CommandCompletedSuccessfullyAckToDequeue
                | CommandCompletedSuccessfullyEndingSession
        )
    }

    /// Returns true if this error is likely to persist across similar requests inside the same
    /// connection or session.
    ///
    /// The same command with different arguments might succeed in some cases.
    pub fn is_persistent(&self) -> bool {
        use ResultCode::*;
        match self {
            // The same command with different arguments will result in the same error
            UnknownCommand
            | CommandSyntaxError
            | RequiredParameterMissing
            | ParameterValueRangeError
            | ParameterValueSyntaxError
            | UnimplementedProtocolVersion
            | UnimplementedCommand
            | UnimplementedOption
            | UnimplementedExtension => true,
            // The connection is in an unhealthy state
            CommandFailedServerClosingConnection
            | AuthenticationErrorServerClosingConnection
            | SessionLimitExceededServerClosingConnection => true,
            _ => false,
        }
    }
}

impl<'xml> FromXml<'xml> for ResultCode {
    fn matches(id: instant_xml::Id<'_>, field: Option<instant_xml::Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut instant_xml::Deserializer<'cx, 'xml>,
    ) -> Result<(), instant_xml::Error> {
        let mut value = None;
        u16::deserialize(&mut value, field, deserializer)?;
        if let Some(value) = value {
            *into = match Self::from_u16(value) {
                Some(value) => Some(value),
                None => {
                    return Err(instant_xml::Error::UnexpectedValue(format!(
                        "unexpected result code '{value}'"
                    )))
                }
            };
        }

        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: instant_xml::Kind = Kind::Scalar;
}

/// Type corresponding to the `<trID>` tag in an EPP response XML
#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "trID", ns(EPP_XMLNS))]
pub struct ResponseTRID {
    /// The client TRID
    #[xml(rename = "clTRID")]
    pub client_tr_id: Option<String>,
    /// The server TRID
    #[xml(rename = "svTRID")]
    pub server_tr_id: String,
}

/// Type corresponding to the `<msgQ>` tag in an EPP response XML
#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "msgQ", ns(EPP_XMLNS))]
pub struct MessageQueue {
    /// The message count
    #[xml(attribute)]
    pub count: u32,
    /// The message ID
    #[xml(attribute)]
    pub id: String,
    /// The message date
    #[xml(rename = "qDate")]
    pub date: Option<DateTime<Utc>>,
    /// The message text
    #[xml(rename = "msg")]
    pub message: Option<Message>,
}

#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "msg", ns(EPP_XMLNS))]
pub struct Message {
    #[xml(attribute)]
    pub lang: Option<String>,
    #[xml(direct)]
    pub text: String,
}

#[derive(Debug, FromXml, PartialEq)]
/// Type corresponding to the `<response>` tag in an EPP response XML
/// containing an `<extension>` tag
#[xml(rename = "response", ns(EPP_XMLNS))]
pub struct Response<D, E> {
    /// Data under the `<result>` tag
    pub result: EppResult,
    /// Data under the `<msgQ>` tag
    #[xml(rename = "msgQ")]
    pub message_queue: Option<MessageQueue>,
    /// Data under the `<resData>` tag
    pub res_data: Option<ResponseData<D>>,
    /// Data under the `<extension>` tag
    pub extension: Option<Extension<E>>,
    /// Data under the `<trID>` tag
    pub tr_ids: ResponseTRID,
}

#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "resData", ns(EPP_XMLNS))]
pub struct ResponseData<D> {
    data: D,
}

impl<D> ResponseData<D> {
    pub fn into_inner(self) -> D {
        self.data
    }
}

#[derive(Debug, FromXml, PartialEq)]
/// Type corresponding to the `<response>` tag in an EPP response XML
/// without `<msgQ>` or `<resData>` sections. Generally used for error handling
#[xml(rename = "response", ns(EPP_XMLNS))]
pub struct ResponseStatus {
    /// Data under the `<result>` tag
    pub result: EppResult,
    #[xml(rename = "trID")]
    /// Data under the `<trID>` tag
    pub tr_ids: ResponseTRID,
}

impl<T, E> Response<T, E> {
    /// Returns the data under the corresponding `<resData>` from the EPP XML
    pub fn res_data(&self) -> Option<&T> {
        match &self.res_data {
            Some(res_data) => Some(&res_data.data),
            None => None,
        }
    }

    pub fn extension(&self) -> Option<&E> {
        match &self.extension {
            Some(extension) => Some(&extension.data),
            None => None,
        }
    }

    /// Returns the data under the corresponding `<msgQ>` from the EPP XML
    pub fn message_queue(&self) -> Option<&MessageQueue> {
        match &self.message_queue {
            Some(queue) => Some(queue),
            None => None,
        }
    }
}

#[derive(Debug, Eq, FromXml, PartialEq)]
#[xml(rename = "extension", ns(EPP_XMLNS))]
pub struct Extension<E> {
    pub data: E,
}

#[cfg(test)]
mod tests {
    use super::{ResponseStatus, ResultCode};
    use crate::tests::{get_xml, CLTRID, SVTRID};
    use crate::xml;

    #[test]
    fn error() {
        let xml = get_xml("response/error.xml").unwrap();
        let object = xml::deserialize::<ResponseStatus>(xml.as_str()).unwrap();

        assert_eq!(object.result.code, ResultCode::ObjectDoesNotExist);
        assert_eq!(object.result.message, "Object does not exist");
        assert_eq!(object.result.ext_values.len(), 1);
        assert_eq!(
            object.result.ext_values[0].reason.text,
            "545 Object not found"
        );
        assert_eq!(object.result.ext_values[0].reason.lang, None);
        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn error_ext() {
        let xml = get_xml("response/error_ext.xml").unwrap();
        let object = xml::deserialize::<ResponseStatus>(xml.as_str()).unwrap();

        assert_eq!(object.result.code, ResultCode::ParameterValuePolicyError);
        assert_eq!(object.result.message, "Parameter value policy error");
        assert_eq!(object.result.ext_values.len(), 1);
        assert_eq!(
            object.result.ext_values[0].reason.text,
            "Maximum of 20 domains exceeded."
        );
        assert_eq!(object.result.ext_values[0].reason.lang, None);

        assert_eq!(object.result.ext_values[0].value.inner.name, "name");
        assert_eq!(
            object.result.ext_values[0].value.inner.ns,
            "urn:ietf:params:xml:ns:domain-1.0"
        );
        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn error_value_attrs() {
        let xml = get_xml("response/error_value_attrs.xml").unwrap();
        let object = xml::deserialize::<ResponseStatus>(xml.as_str()).unwrap();

        assert_eq!(object.result.code, ResultCode::ParameterValuePolicyError);

        let ext_value = &object.result.ext_values[0];

        // anyAttribute on <value>
        assert_eq!(ext_value.value.attributes.len(), 1);
        assert_eq!(ext_value.value.attributes[0].0.as_deref(), Some("s"));
        assert_eq!(ext_value.value.attributes[0].1, "suggestedValue");
        assert_eq!(ext_value.value.attributes[0].2, "pqr-123");

        // inner element
        assert_eq!(ext_value.value.inner.name, "name");
        assert_eq!(
            ext_value.value.inner.ns,
            "urn:ietf:params:xml:ns:domain-1.0"
        );
        assert_eq!(ext_value.value.inner.text.as_deref(), Some("example.com"));

        // lang attribute on <reason>
        assert_eq!(ext_value.reason.lang.as_deref(), Some("de"));
        assert_eq!(ext_value.reason.text, "Domainname ist nicht verfügbar.");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn poll_unhandled_namespace() {
        let xml = get_xml("response/poll_unhandled_namespace.xml").unwrap();
        let object = xml::deserialize::<super::Response<(), ()>>(xml.as_str()).unwrap();

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );
        assert_eq!(object.result.ext_values.len(), 2);

        // First extValue: domain:infData
        let ev0 = &object.result.ext_values[0];
        assert_eq!(ev0.value.inner.name, "infData");
        assert_eq!(ev0.value.inner.ns, "urn:ietf:params:xml:ns:domain-1.0");
        assert_eq!(ev0.value.inner.children.len(), 10);
        // domain:name
        assert_eq!(ev0.value.inner.children[0].name, "name");
        assert_eq!(
            ev0.value.inner.children[0].text.as_deref(),
            Some("domain.example")
        );
        // domain:status has attribute s="ok" and is self-closing
        let status = &ev0.value.inner.children[2];
        assert_eq!(status.name, "status");
        assert_eq!(status.attributes.len(), 1);
        assert_eq!(status.attributes[0].1, "s");
        assert_eq!(status.attributes[0].2, "ok");
        // domain:contact with type attribute
        let contact = &ev0.value.inner.children[4];
        assert_eq!(contact.name, "contact");
        assert_eq!(contact.attributes[0].1, "type");
        assert_eq!(contact.attributes[0].2, "admin");
        assert_eq!(contact.text.as_deref(), Some("sh8013"));

        assert_eq!(
            ev0.reason.text.trim(),
            "urn:ietf:params:xml:ns:domain-1.0 not in login services"
        );

        // Second extValue: changePoll:changeData
        let ev1 = &object.result.ext_values[1];
        assert_eq!(ev1.value.inner.name, "changeData");
        assert_eq!(ev1.value.inner.ns, "urn:ietf:params:xml:ns:changePoll-1.0");
        // state="after" attribute on changeData
        assert_eq!(ev1.value.inner.attributes.len(), 1);
        assert_eq!(ev1.value.inner.attributes[0].1, "state");
        assert_eq!(ev1.value.inner.attributes[0].2, "after");
        assert_eq!(ev1.value.inner.children.len(), 6);
        // changePoll:caseId has type="urs" attribute
        let case_id = &ev1.value.inner.children[4];
        assert_eq!(case_id.name, "caseId");
        assert_eq!(case_id.attributes[0].1, "type");
        assert_eq!(case_id.attributes[0].2, "urs");

        assert_eq!(
            ev1.reason.text.trim(),
            "urn:ietf:params:xml:ns:changePoll-1.0 not in login services"
        );

        // msgQ
        let mq = object.message_queue.unwrap();
        assert_eq!(mq.count, 201);
        assert_eq!(mq.id, "1");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), "ABC-12345");
        assert_eq!(object.tr_ids.server_tr_id, "54322-XYZ");
    }

    #[test]
    fn domain_info_unhandled_namespace() {
        let xml = get_xml("response/domain/info_unhandled_namespace.xml").unwrap();
        let object =
            xml::deserialize::<super::Response<crate::domain::info::InfoData, ()>>(xml.as_str())
                .unwrap();

        assert_eq!(object.result.code, ResultCode::CommandCompletedSuccessfully);

        // extValue with rgp:infData not in login services
        assert_eq!(object.result.ext_values.len(), 1);
        let ev = &object.result.ext_values[0];
        assert_eq!(ev.value.inner.name, "infData");
        assert_eq!(ev.value.inner.ns, "urn:ietf:params:xml:ns:rgp-1.0");
        assert_eq!(ev.value.inner.children.len(), 1);
        let rgp_status = &ev.value.inner.children[0];
        assert_eq!(rgp_status.name, "rgpStatus");
        assert_eq!(rgp_status.attributes.len(), 1);
        assert_eq!(rgp_status.attributes[0].1, "s");
        assert_eq!(rgp_status.attributes[0].2, "redemptionPeriod");
        assert_eq!(
            ev.reason.text,
            "urn:ietf:params:xml:ns:rgp-1.0 not in login services"
        );

        // resData parsed as domain InfoData
        let result = object.res_data().unwrap();
        assert_eq!(result.name, "example.com");
        assert_eq!(result.roid, "EXAMPLE1-REP");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), "ABC-12345");
        assert_eq!(object.tr_ids.server_tr_id, "54322-XYZ");
    }
}
