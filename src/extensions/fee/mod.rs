//! Types for EPP fee request and responses (version 1.0)
//!
//! As described in [Registry Fee Extension for the Extensible Provisioning Protocol](https://datatracker.ietf.org/doc/rfc8748/)

use std::borrow::Cow;
use std::ops::Deref;

use instant_xml::Id;
use instant_xml::{FromXml, ToXml};

use crate::domain::{
    check::DomainCheck, transfer::DomainTransfer, DomainCreate, DomainDelete, DomainRenew,
    DomainUpdate,
};
use crate::extensions::fee::duration::XsdDuration;
use crate::request::{Extension, Transaction};

// Todo: Should this be part of instant_xml?
mod duration;
pub use duration::format_duration;

/// Type for EPP XML `<fee:check>` element
///
/// Used in <check> commands.
///
/// Serializes to:
/// ```xml
/// <fee:check xmlns:fee="urn:ietf:params:xml:ns:epp:fee-1.0">
///   <fee:currency>USD</fee:currency>
///   <fee:command name="create">
///     <fee:period unit="y">2</fee:period>
///   </fee:command>
///   <fee:command name="renew"/>
///   <fee:command name="transfer"/>
///   <fee:command name="restore"/>
/// </fee:check>
/// ```
#[derive(Debug, ToXml, Default)]
#[xml(rename = "check", ns(XMLNS))]
pub struct Check<'a> {
    pub currency: Option<Currency>,
    #[xml(rename = "command")]
    pub commands: Vec<Command<'a>>,
}

impl<'a> Check<'a> {
    /// Create a new Fee Check request
    ///
    /// You can add commands using the `push` method.
    ///
    /// Example:
    /// ```rust
    /// use instant_epp::extensions::fee::{Check, Command, Currency};
    /// let fee_check = Check::new()
    ///   .push(Command::create())
    ///  .push(Command::renew()).with_currency(Currency::Usd);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, command: impl Into<Command<'a>>) -> Self {
        self.commands.push(command.into());
        self
    }

    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }
}

/// Type for EPP XML `<fee:create>` element
///
/// Used in <create> commands.
///
/// Serializes to:
/// ```xml
/// <fee:create xmlns:fee="urn:ietf:params:xml:ns:fee-1.0">
///  <fee:currency>USD</fee:currency>
///  <fee:command phase="sunrise">create</fee:command>
/// </fee:info>
/// ```
#[derive(Debug, ToXml)]
#[xml(rename = "create", ns(XMLNS))]
pub struct Create {
    pub inner: TransformType,
}

impl Create {
    /// Create a new fee create request
    ///
    /// This uses the default currency of the account. To change the currency, use
    /// [`Self::with_currency`].
    ///
    /// # Note
    /// Use the same fee obtained from the check command.
    // Todo: Should we add a From<&CheckData> impl here?
    pub fn new(fee: FeeType) -> Self {
        Self {
            inner: TransformType {
                currency: Default::default(),
                fees: vec![fee],
                credits: Default::default(),
            },
        }
    }

    /// Set the currency for the fee create request
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.inner.currency = Some(currency);
        self
    }
}

/// Type for EPP XML `<fee:renew>` element
///
/// Used in <renew> commands.
///
/// Serializes to:
/// ```xml
/// <fee:renew xmlns:fee="urn:ietf:params:xml:ns:fee-1.0">
///  <fee:currency>USD</fee:currency>
///  <fee:fee>5.00</fee:fee>
/// </fee:info>
/// ```
#[derive(Debug, ToXml)]
#[xml(rename = "renew", ns(XMLNS))]
pub struct Renew {
    pub inner: TransformType,
}

impl Renew {
    /// Create a new fee renew request
    ///
    /// This uses the default currency of the account. To change the currency, use
    /// [`Self::with_currency`].
    ///
    /// # Note
    /// Use the same fee obtained from the check command.
    // Todo: Should we add a From<&CheckData> impl here?
    pub fn new(fee: FeeType) -> Self {
        Self {
            inner: TransformType {
                currency: Default::default(),
                fees: vec![fee],
                credits: Default::default(),
            },
        }
    }

    /// Set the currency for the fee renew request
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.inner.currency = Some(currency);
        self
    }
}

/// Type for EPP XML `<fee:update>` element
///
/// Used in <update> commands.
///
/// Serializes to:
/// ```xml
/// <fee:update xmlns:fee="urn:ietf:params:xml:ns:fee-1.0">
///  <fee:currency>USD</fee:currency>
///  <fee:fee>5.00</fee:fee>
/// </fee:info>
/// ```
#[derive(Debug, ToXml)]
#[xml(rename = "update", ns(XMLNS))]
pub struct Update {
    pub inner: TransformType,
}

impl Update {
    /// Create a new fee update request
    ///
    /// This uses the default currency of the account. To change the currency, use
    /// [`Self::with_currency`].
    ///
    /// # Note
    /// Use the same fee obtained from the check command.
    // Todo: Should we add a From<&CheckData> impl here?
    pub fn new(fee: FeeType) -> Self {
        Self {
            inner: TransformType {
                currency: Default::default(),
                fees: vec![fee],
                credits: Default::default(),
            },
        }
    }

    /// Set the currency for the fee update request
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.inner.currency = Some(currency);
        self
    }
}

/// Type for EPP XML `<fee:transfer>` element
///
/// Used in <transfer> commands.
///
/// This extension adds elements to both the EPP <create> command and
/// response, when the extension has been selected during a <login>
/// command.
///
/// TransferRequest serializes to:
/// ```xml
/// <fee:transfer xmlns:fee="urn:ietf:params:xml:ns:fee-1.0">
///  <fee:currency>USD</fee:currency>
///  <fee:fee>5.00</fee:fee>
/// </fee:info>
/// ```
#[derive(Debug, ToXml)]
#[xml(forward)]
pub enum Transfer {
    // This extension does not add any elements to the EPP <transfer> query
    // command, but does include elements in the response, when the
    // extension has been selected during a <login> command.
    Query(TransferQuery),
    // This extension adds elements to both the EPP <transfer> command and
    // response, when the value of the "op" attribute of the <transfer>
    // command element is "request", and the extension has been selected
    // during the <login> command.
    Request(TransferRequest),
}

impl Transfer {
    /// Create a new fee transfer query
    pub fn query() -> Self {
        Self::Query(TransferQuery)
    }

    /// Create a new fee transfer request
    ///
    /// # Note
    /// Use the same fee obtained from the check command.
    pub fn request(fee: FeeType) -> Self {
        Self::Request(TransferRequest {
            inner: TransformType {
                currency: Default::default(),
                fees: vec![fee],
                credits: Default::default(),
            },
        })
    }
}

/// Type for EPP XML `<fee:transfer>` element in query op
///
/// Used in <transfer> commands.
///
/// Does not add any elements to the request.
#[derive(Debug)]
pub struct TransferQuery;

// TODO: Find solution to avoid these.
// This is needed to avoid writing an empty <delete> tag into <extension>
// instant-epp currently requires ToXml impls for all command types
// and we need a way to make infer the response type added by the extension
// for this command.
// This will still add the <extension> tag currently, but it will be empty.
// Some EPP servers are not happy about en empty <extension> tag though.
impl ToXml for TransferQuery {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _field: Option<instant_xml::Id<'_>>,
        _serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        Ok(())
    }
}

/// Type for EPP XML `<fee:transfer>` element in request op
///
/// Used in <transfer> commands.
#[derive(Debug, ToXml)]
#[xml(rename = "transfer", ns(XMLNS))]
pub struct TransferRequest {
    pub inner: TransformType,
}

/// Inner type for general transform commands
///
/// general transform (create, renew, update, transfer) command
/// See fee:transformCommandType in the spec.
#[derive(Debug, ToXml)]
#[xml(transparent)]
pub struct TransformType {
    pub currency: Option<Currency>,
    #[xml(rename = "fee")]
    pub fees: Vec<FeeType>,
    #[xml(rename = "credit")]
    pub credits: Vec<Credit>,
}

/// Type for EPP XML `<fee:delete>` element
///
/// Used in <delete> commands.
#[derive(Debug)]
pub struct Delete;

// TODO: Find solution to avoid these.
// This is needed to avoid writing an empty <delete> tag into <extension>
// instant-epp currently requires ToXml impls for all command types
// and we need a way to make infer the response type added by the extension
// for this command.
// This will still add the <extension> tag currently, but it will be empty.
// Some EPP servers are not happy about en empty <extension> tag though.
impl ToXml for Delete {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _field: Option<instant_xml::Id<'_>>,
        _serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        Ok(())
    }
}

/// Type for EPP XML `<fee:command>` tag implements fee:commandType.
#[derive(Debug, FromXml, ToXml)]
#[xml(rename = "command", ns(XMLNS))]
pub struct Command<'a> {
    #[xml(attribute)]
    phase: Option<Cow<'a, str>>,
    #[xml(attribute)]
    subphase: Option<Cow<'a, str>>,
    #[xml(attribute, rename = "customName")]
    custom_name: Option<Cow<'a, str>>,
    #[xml(attribute)]
    name: CommandEnum,
    #[xml(direct, rename = "period")]
    period: Option<PeriodType>,
}

impl<'a> Command<'a> {
    /// Create a `<fee:command>` for domain creation
    pub fn create() -> Self {
        Command {
            name: CommandEnum::Create,
            phase: None,
            subphase: None,
            custom_name: None,
            period: None,
        }
    }

    /// Create a `<fee:command>` for domain renewal
    pub fn renew() -> Self {
        Command {
            name: CommandEnum::Renew,
            phase: None,
            subphase: None,
            custom_name: None,
            period: None,
        }
    }

    /// Create a `<fee:command>` for domain transfer
    pub fn transfer() -> Self {
        Command {
            name: CommandEnum::Transfer,
            phase: None,
            subphase: None,
            custom_name: None,
            period: None,
        }
    }

    /// Create a `<fee:command>` for domain restoration
    pub fn restore() -> Self {
        Command {
            name: CommandEnum::Restore,
            phase: None,
            subphase: None,
            custom_name: None,
            period: None,
        }
    }

    pub fn with_phase(mut self, phase: impl Into<Cow<'a, str>>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_subphase(mut self, subphase: impl Into<Cow<'a, str>>) -> Self {
        self.subphase = Some(subphase.into());
        self
    }

    pub fn with_custom_name(mut self, custom_name: impl Into<Cow<'a, str>>) -> Self {
        self.custom_name = Some(custom_name.into());
        self
    }

    pub fn with_period(mut self, period: impl Into<PeriodType>) -> Self {
        self.period = Some(period.into());
        self
    }

    pub fn phase(&self) -> Option<&str> {
        self.phase.as_deref()
    }

    pub fn subphase(&self) -> Option<&str> {
        self.subphase.as_deref()
    }

    pub fn command(&self) -> CommandEnum {
        self.name
    }
}

/// Type for EPP XML `<fee:chkData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "chkData", ns(XMLNS))]
pub struct CheckData {
    pub currency: Currency,
    #[xml(rename = "cd")]
    pub data: Vec<ObjectCDType>,
}

/// Type for EPP XML `<fee:cd>` tag implements fee:objectCDType
#[derive(Debug, FromXml)]
#[xml(rename = "cd", ns(XMLNS))]
pub struct ObjectCDType {
    /// Defaults to true.
    ///
    /// If "avail" is false, then the `<fee:cd>` or the `<fee:command>` element MUST contain a
    /// `<fee:reason>` element (as described in Section 3.9), and the server MAY eliminate some
    /// or all of the `<fee:command>` element(s).
    // Todo: Make this non-optional with default true, once instant-xml supports default values.
    #[xml(attribute)]
    pub avail: Option<bool>,
    /// The object identifier, e.g domain references in the <check> command
    #[xml(rename = "objID")]
    pub obj_id: String,
    pub class: Option<String>,
    pub command: Vec<CommandDataType>,
    pub reason: Option<ReasonType>,
}

/// Type for EPP XML `<fee:reason>` tag implements fee:reasonType.
///
/// Provides server-specific text in an effort to better explain why
/// a <check> command did not complete as the client expected.
#[derive(Debug, Clone, FromXml)]
#[xml(rename = "reason", ns(XMLNS))]
pub struct ReasonType {
    #[xml(attribute)]
    pub lang: Option<String>,
    #[xml(direct)]
    pub description: String,
}

/// Type for EPP XML `<fee:creData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "creData", ns(XMLNS))]
pub struct CreateData {
    pub inner: TransformResultType,
}

impl Deref for CreateData {
    type Target = TransformResultType;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Type for EPP XML `<fee:renData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "renData", ns(XMLNS))]
pub struct RenewData {
    pub inner: TransformResultType,
}

impl Deref for RenewData {
    type Target = TransformResultType;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Type for EPP XML `<fee:updData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "updData", ns(XMLNS))]
pub struct UpdateData {
    inner: TransformResultType,
}

impl Deref for UpdateData {
    type Target = TransformResultType;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Type for EPP XML `<fee:trnData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "trnData", ns(XMLNS))]
pub struct TransferData {
    pub inner: TransformResultType,
}

impl Deref for TransferData {
    type Target = TransformResultType;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Type for EPP XML `<fee:delData>` extension response
#[derive(Debug, FromXml)]
#[xml(rename = "delData", ns(XMLNS))]
pub struct DeleteData {
    pub inner: TransformResultType,
}

/// Inner type for transform responses fee:transformResultType
///
/// general transform (create, renew, update) result
#[derive(Debug)]
pub struct TransformResultType {
    pub currency: Option<Currency>,
    pub period: Option<PeriodType>,
    pub fees: Vec<FeeType>,
    pub credit: Vec<Credit>,
    pub balance: Option<Balance>,
    pub credit_limit: Option<CreditLimit>,
}

// We need to implement FromXml manually here because FromXml currently contains bugs.
// See https://github.com/djc/instant-xml/issues/113
// Even when used with named fields this the derive macro results in
// `Xml(DuplicateValue("TransformResultType::currency"))``
impl<'xml> FromXml<'xml> for TransformResultType {
    #[inline]
    fn matches(id: Id<'_>, _: Option<Id<'_>>) -> bool {
        <Option<Currency> as FromXml<'xml>>::matches(id, None)
            || <Option<PeriodType> as FromXml<'xml>>::matches(id, None)
            || <Vec<FeeType> as FromXml<'xml>>::matches(
                id,
                Some(Id {
                    ns: XMLNS,
                    name: "fee",
                }),
            )
            || <Vec<Credit> as FromXml<'xml>>::matches(
                id,
                Some(Id {
                    ns: XMLNS,
                    name: "credit",
                }),
            )
            || <Option<Balance> as FromXml<'xml>>::matches(id, None)
            || <Option<CreditLimit> as FromXml<'xml>>::matches(id, None)
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut instant_xml::Deserializer<'cx, 'xml>,
    ) -> Result<(), instant_xml::Error> {
        let current = deserializer.parent();
        // mode scalar will emit code that matches on the field, but the default transparent does not forward the field.
        const CURRENCY_TAG: Id = Id {
            ns: XMLNS,
            name: "currency",
        };
        if <Option<Currency> as FromXml<'xml>>::matches(current, Some(CURRENCY_TAG)) {
            <Option<Currency> as FromXml>::deserialize(
                &mut into.currency,
                "Transform::currency",
                deserializer,
            )?;
            deserializer.ignore()?;
        } else if <Option<PeriodType> as FromXml<'xml>>::matches(current, None) {
            <Option<PeriodType> as FromXml>::deserialize(
                &mut into.period,
                "Transform::period",
                deserializer,
            )?;
        } else if <Vec<FeeType> as FromXml<'xml>>::matches(
            current,
            Some(Id {
                ns: XMLNS,
                name: "fee",
            }),
        ) {
            <Vec<FeeType> as FromXml>::deserialize(
                &mut into.fees,
                "Transform::fees",
                deserializer,
            )?;
        } else if <Option<Balance> as FromXml<'xml>>::matches(current, None) {
            <Option<Balance> as FromXml>::deserialize(
                &mut into.balance,
                "Transform::balance",
                deserializer,
            )?;
        } else if <Vec<Credit> as FromXml<'xml>>::matches(current, None) {
            <Vec<Credit> as FromXml>::deserialize(
                &mut into.credit,
                "Transform::credit",
                deserializer,
            )?;
        } else if <Option<CreditLimit> as FromXml<'xml>>::matches(current, None) {
            <Option<CreditLimit> as FromXml>::deserialize(
                &mut into.credit_limit,
                "Transform::credit_limit",
                deserializer,
            )?;
        }
        Ok(())
    }

    type Accumulator = TransformAccumulator<'xml>;
    const KIND: instant_xml::Kind = instant_xml::Kind::Element;
}

#[derive(Default)]
pub struct TransformAccumulator<'xml> {
    currency: <Option<Currency> as FromXml<'xml>>::Accumulator,
    fees: <Vec<FeeType> as FromXml<'xml>>::Accumulator,
    period: <Option<PeriodType> as FromXml<'xml>>::Accumulator,
    credit: <Vec<Credit> as FromXml<'xml>>::Accumulator,
    balance: <Option<Balance> as FromXml<'xml>>::Accumulator,
    credit_limit: <Option<CreditLimit> as FromXml<'xml>>::Accumulator,
}

impl<'xml> instant_xml::Accumulate<TransformResultType> for TransformAccumulator<'xml> {
    fn try_done(self, _: &'static str) -> Result<TransformResultType, instant_xml::Error> {
        Ok(TransformResultType {
            currency: self.currency.try_done("Transform::currency")?,
            fees: self.fees.try_done("Transform::fees")?,
            period: self.period.try_done("Transform::period")?,
            credit: self.credit.try_done("Transform::credit")?,
            balance: self.balance.try_done("Transform::balance")?,
            credit_limit: self.credit_limit.try_done("Transform::credit_limit")?,
        })
    }
}

/// Type for EPP XML `<fee:command>` tag implements fee:commandDataType.
#[derive(Debug, Clone, FromXml)]
#[xml(rename = "command", ns(XMLNS))]
pub struct CommandDataType {
    #[xml(attribute)]
    pub phase: Option<String>,
    #[xml(attribute)]
    pub subphase: Option<String>,
    #[xml(attribute, rename = "customName")]
    pub custom_name: Option<String>,
    #[xml(attribute)]
    pub name: CommandEnum,
    /// This should default to false if not present
    #[xml(attribute)]
    pub standard: Option<bool>,
    #[xml(rename = "period")]
    pub period: Option<PeriodType>,
    #[xml(rename = "fee")]
    pub fees: Vec<FeeType>,
    #[xml(rename = "credit")]
    pub credits: Vec<Credit>,
    pub reason: Option<ReasonType>,
}

/// Type for EPP XML `<fee:balance>` tag
///
/// Used in <create>, <update>, <renew>, <transfer> and <delete> responses
#[derive(Debug, FromXml)]
#[xml(rename = "balance", ns(XMLNS))]
pub struct Balance {
    #[xml(direct)]
    pub amount: f64,
}

/// Type for EPP XML `<fee:creditLimit>` tag
///
/// Used in <create>, <update>, <renew>, <transfer> and <delete> responses
#[derive(Debug, FromXml)]
#[xml(rename = "creditLimit", ns(XMLNS))]
pub struct CreditLimit {
    #[xml(direct)]
    pub amount: f64,
}

/// Type for EPP XML `<fee:credit>` tag implements fee:creditType
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "credit", ns(XMLNS))]
pub struct Credit {
    #[xml(attribute)]
    pub description: Option<String>,
    #[xml(direct)]
    pub amount: f64,
}

/// Type for EPP XML `<fee:fee>` tag implementing type fee:feeType
#[derive(Debug, Clone, PartialEq, FromXml)]
#[xml(rename = "fee", ns(XMLNS))]
pub struct FeeType {
    #[xml(attribute)]
    pub description: Option<String>,
    #[xml(attribute)]
    pub refundable: Option<bool>,
    #[xml(attribute, rename = "grace-period")]
    pub grace_period: Option<XsdDuration>,
    #[xml(attribute)]
    pub applied: Option<Applied>,
    #[xml(direct)]
    pub amount: f64,
}

// We need a custom ToXml to emit the decimal amount correctly
// There seems to be an incompatibility with `serialize_with` and `direct`
// We need direct for the FromXml derive to work correctly, but you cannot
// combine this with `serialize_with`.
impl ToXml for FeeType {
    fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
        &self,
        _field: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let prefix = serializer.write_start("fee", XMLNS)?;
        let new = instant_xml::ser::Context::<0usize> {
            default_ns: XMLNS,
            ..Default::default()
        };

        let old = serializer.push(new)?;
        if self.description.present() {
            serializer.write_attr("description", XMLNS, &self.description)?;
        }
        if self.refundable.present() {
            serializer.write_attr("refundable", XMLNS, &self.refundable)?;
        }
        if self.grace_period.present() {
            serializer.write_attr("grace-period", XMLNS, &self.grace_period)?;
        }
        if self.applied.present() {
            serializer.write_attr("applied", XMLNS, &self.applied)?;
        }
        serializer.end_start()?;
        // decimal type requires at least one digit after the decimal point, we use two, as this is a currency.
        // Todo, this should use a proper decimal type.
        let amount_str = format!("{:.2}", self.amount);
        serializer.write_str(&amount_str)?;
        serializer.write_close(prefix, "fee")?;
        serializer.pop(old);
        Ok(())
    }
}

/// Scalar enum for fee:applied
#[derive(Debug, Clone, Copy, Default, PartialEq, FromXml, ToXml)]
#[xml(scalar, rename_all = "lowercase")]
pub enum Applied {
    #[default]
    Immediate,
    Delayed,
}

/// Scalar enum for fee:command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromXml, ToXml)]
#[xml(scalar, rename_all = "lowercase")]
pub enum CommandEnum {
    Create,
    Renew,
    Transfer,
    Restore,
}

/// Scalar enum for fee:currency
#[derive(Debug, Clone, Copy, PartialEq, FromXml, ToXml)]
#[xml(scalar, rename = "currency", rename_all = "UPPERCASE", ns(XMLNS))]
pub enum Currency {
    Usd,
    Eur,
    Gbp,
}

/// Type for EPP XML `<fee:period>` tag
#[derive(Debug, Clone, FromXml, ToXml)]
#[xml(rename = "period", ns(XMLNS))]
pub struct PeriodType {
    #[xml(attribute)]
    unit: String,
    #[xml(direct)]
    value: u32,
}

impl PeriodType {
    /// Create a PeriodType in years
    pub fn years(value: u32) -> Self {
        Self {
            unit: "y".to_string(),
            value,
        }
    }

    /// Get the unit of the period
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Get the value of the period
    pub fn value(&self) -> u32 {
        self.value
    }
}

impl From<u32> for PeriodType {
    /// Convert u32 to PeriodType in years
    fn from(value: u32) -> Self {
        Self::years(value)
    }
}

impl Transaction<Check<'_>> for DomainCheck<'_> {}
impl Transaction<Update> for DomainUpdate<'_> {}
impl Transaction<Create> for DomainCreate<'_> {}
impl Transaction<Renew> for DomainRenew<'_> {}
impl Transaction<Delete> for DomainDelete<'_> {}
impl Transaction<Transfer> for DomainTransfer<'_> {}

impl Extension for Check<'_> {
    type Response = CheckData;
}

impl Extension for Transfer {
    type Response = TransferData;
}

impl Extension for Create {
    type Response = CreateData;
}

impl Extension for Update {
    type Response = UpdateData;
}

impl Extension for Renew {
    type Response = RenewData;
}

impl Extension for Delete {
    type Response = DeleteData;
}

pub const XMLNS: &str = "urn:ietf:params:xml:ns:epp:fee-1.0";

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::NaiveDate;

    use crate::domain::transfer::DomainTransfer;
    use crate::domain::update::DomainChangeInfo;
    use crate::domain::{DomainContact, HostInfo, HostObj, Period, PeriodLength};
    use crate::tests::{assert_serialized, response_from_file_with_ext};

    use super::*;

    #[test]
    fn request_check() {
        assert_serialized(
            "request/extensions/fee/check.xml",
            (
                &DomainCheck {
                    domains: &["example.com", "example.net", "example.xyz"],
                },
                &Check::new()
                    .with_currency(Currency::Usd)
                    .push(Command::create().with_period(2))
                    .push(Command::renew())
                    .push(Command::transfer())
                    .push(Command::restore()),
            ),
        );
    }

    #[test]
    fn request_create() {
        let ns = vec![
            HostInfo::Obj(HostObj {
                name: "ns1.example.net".into(),
            }),
            HostInfo::Obj(HostObj {
                name: "ns2.example.net".into(),
            }),
        ];

        let contacts = vec![
            DomainContact {
                contact_type: "admin".into(),
                id: "sh8013".into(),
            },
            DomainContact {
                contact_type: "tech".into(),
                id: "sh8013".into(),
            },
        ];

        let create = DomainCreate::new(
            "example.com",
            Period::Years(PeriodLength::new(2).unwrap()),
            Some(&ns),
            Some("jd1234"),
            "2fooBAR",
            Some(&contacts),
        );

        assert_serialized(
            "request/extensions/fee/create.xml",
            (
                &create,
                &Create::new(FeeType {
                    description: None,
                    refundable: None,
                    grace_period: None,
                    applied: None,
                    amount: 5.00,
                })
                .with_currency(Currency::Usd),
            ),
        );
    }

    #[test]
    fn request_delete() {
        assert_serialized(
            "request/extensions/fee/delete.xml",
            (&DomainDelete::new("example.com"), &Delete),
        );
    }

    #[test]
    fn request_renew() {
        let renew = DomainRenew::new(
            "example.com",
            NaiveDate::from_ymd_opt(2000, 4, 3).unwrap(),
            Period::Years(PeriodLength::new(5).unwrap()),
        );

        assert_serialized(
            "request/extensions/fee/renew.xml",
            (
                &renew,
                &Renew {
                    inner: TransformType {
                        currency: Some(Currency::Usd),
                        fees: vec![FeeType {
                            description: None,
                            refundable: None,
                            grace_period: None,
                            applied: None,
                            amount: 5.00,
                        }],
                        credits: vec![],
                    },
                },
            ),
        );
    }

    #[test]
    fn request_transfer_request() {
        let transfer = DomainTransfer::new(
            "example.com",
            Some(Period::Years(PeriodLength::new(1).unwrap())),
            "2fooBAR",
        );

        assert_serialized(
            "request/extensions/fee/transfer_request.xml",
            (
                &transfer,
                &Transfer::Request(TransferRequest {
                    inner: TransformType {
                        currency: Some(Currency::Usd),
                        fees: vec![FeeType {
                            description: None,
                            refundable: None,
                            grace_period: None,
                            applied: None,
                            amount: 5.00,
                        }],
                        credits: vec![],
                    },
                }),
            ),
        );
    }

    #[test]
    fn request_update() {
        let chg = DomainChangeInfo {
            registrant: Some("sh8013"),
            auth_info: None,
        };

        let mut update = DomainUpdate::new("example.com");
        update.info(chg);

        assert_serialized(
            "request/extensions/fee/update.xml",
            (
                &update,
                &Update {
                    inner: TransformType {
                        currency: Some(Currency::Usd),
                        fees: vec![FeeType {
                            description: None,
                            refundable: None,
                            grace_period: None,
                            applied: None,
                            amount: 5.00,
                        }],
                        credits: vec![],
                    },
                },
            ),
        );
    }

    #[test]
    fn response_check() {
        let object =
            response_from_file_with_ext::<DomainCheck, Check>("response/extensions/fee/check.xml");
        let ext = object.extension.unwrap().data;

        assert_eq!(ext.currency, Currency::Usd);

        let results = ext
            .data
            .into_iter()
            .map(|entry| {
                let command_map = entry
                    .command
                    .iter()
                    .cloned()
                    .map(|cmd| (cmd.name, cmd))
                    .collect::<HashMap<_, _>>();

                (entry.obj_id.clone(), (entry, command_map))
            })
            .collect::<HashMap<_, _>>();

        let cd = results.get("example.com").unwrap();

        assert!(cd.0.avail.unwrap());
        assert_eq!(cd.0.class.as_ref().unwrap(), "Premium");
        let command = cd.1.get(&CommandEnum::Create).unwrap();
        assert_eq!(command.period.as_ref().unwrap().value, 2);
        assert_eq!(command.period.as_ref().unwrap().unit, "y");
        assert_eq!(command.fees.len(), 1);
        assert_eq!(
            command.fees,
            vec![FeeType {
                grace_period: Some(XsdDuration::new(0, (5 * 24 * 60 * 60) as f64).unwrap()), // 5 days
                applied: None,
                refundable: Some(true),
                description: Some("Registration Fee".to_string()),
                amount: 10.00
            },]
        );
        let command = cd.1.get(&CommandEnum::Renew).unwrap();

        assert_eq!(command.period.as_ref().unwrap().value, 1);
        assert_eq!(command.period.as_ref().unwrap().unit, "y");
        assert_eq!(command.fees.len(), 1);
        assert_eq!(
            command.fees,
            vec![FeeType {
                grace_period: Some(XsdDuration::new(0, (5 * 24 * 60 * 60) as f64).unwrap()), // 5 days
                applied: None,
                refundable: Some(true),
                description: Some("Renewal Fee".to_string()),
                amount: 10.00
            },]
        );

        let cd = results.get("example.xyz").unwrap();
        assert!(!cd.0.avail.unwrap());
        let command = cd.1.get(&CommandEnum::Create).unwrap();
        assert_eq!(command.period.as_ref().unwrap().value, 2);
        assert_eq!(command.period.as_ref().unwrap().unit, "y");
        assert_eq!(
            command
                .reason
                .as_ref()
                .expect("expected reason")
                .description,
            "Only 1 year registration periods are valid."
        );
    }

    #[test]
    fn response_create() {
        let object = response_from_file_with_ext::<DomainCreate, Create>(
            "response/extensions/fee/create.xml",
        );
        let ext = object.extension().unwrap();
        assert_eq!(ext.currency, Some(Currency::Usd));
        assert_eq!(ext.fees[0].amount, 5.00);
        assert_eq!(
            ext.fees[0].grace_period,
            Some(XsdDuration::new(0, (5 * 24 * 60 * 60) as f64).unwrap()) // 5 days
        );
        assert_eq!(ext.balance.as_ref().unwrap().amount, -5.00);
        assert_eq!(ext.credit_limit.as_ref().unwrap().amount, 1000.00);
    }

    #[test]
    fn response_renew() {
        let object =
            response_from_file_with_ext::<DomainRenew, Renew>("response/extensions/fee/renew.xml");
        let ext = object.extension().unwrap();
        assert_eq!(ext.inner.currency, Some(Currency::Usd));
        assert_eq!(ext.inner.fees[0].amount, 5.00);
        assert_eq!(ext.inner.balance.as_ref().unwrap().amount, 1000.00);
    }

    #[test]
    fn response_delete() {
        let object = response_from_file_with_ext::<DomainDelete, Delete>(
            "response/extensions/fee/delete.xml",
        );

        let ext = object.extension().unwrap();
        assert_eq!(ext.inner.currency, Some(Currency::Usd));
        assert_eq!(ext.inner.credit[0].amount, -5.00);
        assert_eq!(
            ext.inner.credit[0].description.as_ref().unwrap(),
            "AGP Credit"
        );
        assert_eq!(ext.inner.balance.as_ref().unwrap().amount, 1005.00);
    }

    #[test]
    fn response_transfer_query() {
        let object = response_from_file_with_ext::<DomainTransfer, Transfer>(
            "response/extensions/fee/transfer_query.xml",
        );
        let ext = object.extension().unwrap();
        assert_eq!(ext.currency, Some(Currency::Usd));
        assert_eq!(ext.period.as_ref().unwrap().value, 1);
        assert_eq!(ext.fees[0].amount, 5.00);
    }

    #[test]
    fn response_transfer_request() {
        // Same structure as query but different values potentially
        let object = response_from_file_with_ext::<DomainTransfer, Transfer>(
            "response/extensions/fee/transfer_request.xml",
        );
        let ext = object.extension().unwrap();
        assert_eq!(ext.currency, Some(Currency::Usd));
        assert!(ext.period.is_none());
        assert_eq!(ext.fees[0].amount, 5.00);
        assert_eq!(
            ext.fees[0].grace_period,
            Some(XsdDuration::new(0, (5 * 24 * 60 * 60) as f64).unwrap()) //5 days
        );
    }

    #[test]
    fn response_update() {
        let object = response_from_file_with_ext::<DomainUpdate, Update>(
            "response/extensions/fee/update.xml",
        );
        let ext = object.extension().unwrap();
        assert_eq!(ext.currency, Some(Currency::Usd));
        assert_eq!(ext.fees[0].amount, 5.00);
    }
}
