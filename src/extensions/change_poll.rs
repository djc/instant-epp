//! Types for the EPP change poll extention
//!
//! As described in RFC8590: [Change Poll Extension for the Extensible Provisioning Protocol (EPP)](https://www.rfc-editor.org/rfc/rfc8590.html).
//! Tests cases in `tests/resources/response/extensions/changepoll`` are taken from the RFC.

use std::borrow::Cow;

use instant_xml::{Error, FromXml, ToXml};

use crate::response::ConnectionExtensionResponse;

pub const XMLNS: &str = "urn:ietf:params:xml:ns:changePoll-1.0";

impl ConnectionExtensionResponse for ChangePoll<'_> {}

/// Type for EPP XML `<changePoll>` extension
///
/// Attributes associated with the change
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "changeData", ns(XMLNS))]
pub struct ChangePoll<'a> {
    /// Transform operation executed on the object
    pub operation: Operation<'a>,
    /// Date and time when the operation was executed
    pub date: Cow<'a, str>,
    /// Server transaction identifier of the operation
    #[xml(rename = "svTRID")]
    pub server_tr_id: Cow<'a, str>,
    /// Who executed the operation
    pub who: Cow<'a, str>,
    /// Case identifier associated with the operation
    pub case_id: Option<CaseIdentifier<'a>>,
    /// Reason for executing the operation
    pub reason: Option<Reason>,
    /// Enumerated state of the object in the poll message
    #[xml(attribute)]
    // todo: State should utilize the Default impl,
    // but instant-xml does not support it yet.
    state: Option<State>,
}

impl ChangePoll<'_> {
    /// State reflects if the `infData` describes the object before or after the operation
    pub fn state(&self) -> State {
        self.state.unwrap_or_default()
    }
}

/// Transform operation type for `<changePoll:operation>`
// todo: Allow struct enum variants with #[xml(attribute, rename = "op")] in instant-xml,
// to make this struct more ergonomic.
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "operation", ns(XMLNS))]
pub struct Operation<'a> {
    /// Custom value for`OperationKind::Custom`
    #[xml(attribute, rename = "op")]
    op: Option<Cow<'a, str>>,
    /// The operation
    #[xml(direct)]
    kind: OperationType,
}

impl Operation<'_> {
    pub fn kind(&self) -> Result<OperationKind, Error> {
        Ok(match self.kind {
            OperationType::Create => OperationKind::Create,
            OperationType::Delete => OperationKind::Delete,
            OperationType::Renew => OperationKind::Renew,
            OperationType::Transfer => OperationKind::Transfer,
            OperationType::Update => OperationKind::Update,
            OperationType::Restore => OperationKind::Restore,
            OperationType::AutoRenew => OperationKind::AutoRenew,
            OperationType::AutoDelete => OperationKind::AutoDelete,
            OperationType::AutoPurge => OperationKind::AutoPurge,
            OperationType::Custom => match self.op.as_deref() {
                Some(op) => OperationKind::Custom(op),
                None => {
                    return Err(Error::Other(
                        "invariant error: Missing op attribute for custom operation".to_string(),
                    ))
                }
            },
        })
    }
}

/// Enumerated list of operations
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OperationKind<'a> {
    Create,
    Delete,
    Renew,
    Transfer,
    Update,
    Restore,
    AutoRenew,
    AutoDelete,
    AutoPurge,
    Custom(&'a str),
}

/// Internal Enumerated list of operations, with extensibility via "custom"
// See todo on `Operation` struct for reason why this is internal only.
#[derive(Debug, Copy, Clone, FromXml, ToXml)]
#[xml(scalar, rename_all = "camelCase", ns(XMLNS))]
enum OperationType {
    Create,
    Delete,
    Renew,
    Transfer,
    Update,
    Restore,
    AutoRenew,
    AutoDelete,
    AutoPurge,
    Custom,
}

/// Case identifier type for `<changePoll:caseId>`
// todo: Allow struct enum variants with #[xml(attribute, rename = "op")] in instant-xml,
// to make this struct more ergonomic.
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "caseId", ns(XMLNS))]
pub struct CaseIdentifier<'a> {
    #[xml(attribute, rename = "type")]
    id_type: CaseIdentifierType,
    #[xml(attribute)]
    name: Option<Cow<'a, str>>,
    #[xml(direct)]
    pub id: Cow<'a, str>,
}

impl CaseIdentifier<'_> {
    pub fn kind(&self) -> Result<CaseIdentifierKind, Error> {
        Ok(match self.id_type {
            CaseIdentifierType::Udrp => CaseIdentifierKind::Udrp,
            CaseIdentifierType::Urs => CaseIdentifierKind::Urs,
            CaseIdentifierType::Custom => match self.name.as_deref() {
                Some(name) => CaseIdentifierKind::Custom(name),
                None => {
                    return Err(Error::Other(
                        "invariant error: Missing name attribute for custom case identifier"
                            .to_string(),
                    ))
                }
            },
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CaseIdentifierKind<'a> {
    Udrp,
    Urs,
    Custom(&'a str),
}

/// Internal enumerated list of case identifier types
// See todo on `CaseIdentifier` struct for reason why this is internal only.
#[derive(Debug, Copy, Clone, FromXml, ToXml)]
#[xml(scalar, rename_all = "camelCase")]
enum CaseIdentifierType {
    Udrp,
    Urs,
    Custom,
}

/// Reason type for `<changePoll:reason>`
///
/// A human-readable message that describes the reason for the encapsulating element.
/// The language of the response is identified via the "lang" attribute.
///
/// Schema defined in the `eppcom-1.0` XML schema
// todo: while this is defined in `eppcom` schema, it is used with different
// namespaces in additional specs (for example in RFC8590).
// Currently, instant-xml strongly ties namespaces to schemas and does not allow
// a way out of it for this particular case.
#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(rename = "reason", ns(XMLNS))]
pub struct Reason {
    /// The language of the response. If not specified, assume "en" (English).
    #[xml(attribute)]
    pub lang: Option<String>,
    #[xml(direct)]
    pub inner: String,
}

/// Enumerated state of the object in the poll message
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(scalar, rename_all = "camelCase")]
pub enum State {
    Before,
    #[default]
    After,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::NoExtension;
    use crate::poll::Poll;
    use crate::response::ResultCode;
    use crate::tests::{response_from_file_with_ext, CLTRID, SVTRID};

    #[test]
    fn urs_lock_before() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/urs_lock_before.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        let ext = &object.connection_extension().unwrap();

        assert_eq!(ext.state.unwrap(), State::Before);
        assert_eq!(ext.operation.kind().unwrap(), OperationKind::Update);
        assert_eq!(ext.date, "2013-10-22T14:25:57.0Z");
        assert_eq!(ext.server_tr_id, "12345-XYZ");
        assert_eq!(ext.who, "URS Admin");
        assert_eq!(
            ext.case_id.as_ref().unwrap().kind().unwrap(),
            CaseIdentifierKind::Urs
        );
        assert_eq!(ext.reason.as_ref().unwrap().inner, "URS Lock");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn urs_lock_after() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/urs_lock_after.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        let ext = &object.connection_extension().unwrap();

        assert_eq!(ext.state.unwrap(), State::After);

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn custom_sync_after() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/custom_sync_after.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        let ext = &object.connection_extension().unwrap();

        assert_eq!(ext.operation.kind().unwrap(), OperationKind::Custom("sync"));
        assert_eq!(ext.who, "CSR");
        assert_eq!(ext.reason.as_ref().unwrap().inner, "Customer sync request");

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn delete_before() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/delete_before.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn autopurge_before() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/autopurge_before.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }

    #[test]
    fn update_after() {
        let object = response_from_file_with_ext::<Poll, NoExtension, ChangePoll>(
            "response/extensions/change_poll/update_after.xml",
        );

        assert_eq!(
            object.result.code,
            ResultCode::CommandCompletedSuccessfullyAckToDequeue
        );
        assert_eq!(
            object.result.message,
            "Command completed successfully; ack to dequeue"
        );

        assert_eq!(object.tr_ids.client_tr_id.unwrap(), CLTRID);
        assert_eq!(object.tr_ids.server_tr_id, SVTRID);
    }
}
