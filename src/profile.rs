use std::any::Any;

use instant_xml::ser::Context;
use instant_xml::{
    Accumulate, Deserializer, Error, FromXml, FromXmlOwned, Id, Kind, Serializer, ToXml,
};

use crate::common::EPP_XMLNS;
use crate::response::Response;

/// Maps a command and request-extension tuple, for a given registry profile, to
/// the response and the tuple of response extensions it may carry here.
pub trait Profile<Cmd, ReqExts> {
    /// The `<resData>` payload (usually just `Cmd::Response`).
    type Response: FromXmlOwned;
    /// The decoded `<extension>` payload — typically an [`Exts`] tuple.
    type RespExts: FromXmlOwned;
}

/// A request-extension tuple that can be serialized into the `<extension>`
/// element.
///
/// Implemented for tuples up to arity 3. A local trait is used (rather than
/// `ToXml` on bare tuples) because orphan rules forbid implementing the foreign
/// `ToXml` trait for foreign tuple types.
pub trait RequestExts {
    /// Whether the set is empty; when `true` the `<extension>` element is omitted.
    const IS_EMPTY: bool;

    /// Serialize each extension as a child of the already-open `<extension>`.
    fn serialize_into<W: std::fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error>;
}

impl RequestExts for () {
    const IS_EMPTY: bool = true;

    fn serialize_into<W: std::fmt::Write + ?Sized>(
        &self,
        _: &mut Serializer<W>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! impl_request_exts {
    ($($T:ident . $idx:tt),+) => {
        impl<$($T: ToXml),+> RequestExts for ($($T,)+) {
            const IS_EMPTY: bool = false;

            fn serialize_into<W: std::fmt::Write + ?Sized>(
                &self,
                serializer: &mut Serializer<W>,
            ) -> Result<(), Error> {
                $( self.$idx.serialize(None, serializer)?; )+
                Ok(())
            }
        }
    };
}

impl_request_exts!(A.0);
impl_request_exts!(A.0, B.1);
impl_request_exts!(A.0, B.1, C.2);
// Todo: do we need more?

/// The `<command>` document for the profiled path.
///
/// Mirrors [`CommandWrapper`](crate::request::CommandWrapper), but serializes a
/// [`RequestExts`] tuple into a single `<extension>` element instead of one
/// optional extension.
/// TODO: replace CommandWrapper with this.
pub(crate) struct ProfiledCommand<'a, Cmd, ReqExts> {
    pub(crate) command: &'a Cmd,
    pub(crate) exts: &'a ReqExts,
    pub(crate) client_tr_id: &'a str,
}

impl<Cmd: ToXml, ReqExts: RequestExts> ToXml for ProfiledCommand<'_, Cmd, ReqExts> {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let command = serializer.write_start("command", EPP_XMLNS, None::<Context<0>>)?;
        serializer.end_start()?;
        self.command.serialize(None, serializer)?;

        // This bit is important for strict EPP servers.
        if !ReqExts::IS_EMPTY {
            let extension = serializer.write_start("extension", EPP_XMLNS, None::<Context<0>>)?;
            serializer.end_start()?;
            self.exts.serialize_into(serializer)?;
            serializer.write_close(extension)?;
        }

        let cl_tr_id = serializer.write_start("clTRID", EPP_XMLNS, None::<Context<0>>)?;
        serializer.end_start()?;
        serializer.write_str(self.client_tr_id)?;
        serializer.write_close(cl_tr_id)?;

        serializer.write_close(command)?;
        Ok(())
    }
}

/// Decoded `<extension>` payload
///
/// Should store a tuple of optional response extensions.
#[derive(Debug, PartialEq)]
pub struct Exts<T>(pub T);

/// Accumulator for [`Exts`]. A dedicated local type is required because orphan
/// rules forbid implementing the foreign `Accumulate` trait for a bare tuple.
pub struct ExtsAcc<T>(T);

impl<T: Default> Default for ExtsAcc<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<'xml, A: FromXml<'xml>> FromXml<'xml> for Exts<(Option<A>,)> {
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        A::matches(id, field)
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        let id = deserializer.parent();
        if A::matches(id, None) {
            A::deserialize(&mut into.0 .0, field, deserializer)?;
        } else {
            deserializer.ignore()?;
        }
        Ok(())
    }

    type Accumulator = ExtsAcc<(A::Accumulator,)>;
    const KIND: Kind = Kind::Element;
}

impl<'xml, A: FromXml<'xml>> Accumulate<Exts<(Option<A>,)>> for ExtsAcc<(A::Accumulator,)> {
    fn try_done(self, field: &'static str) -> Result<Exts<(Option<A>,)>, Error> {
        Ok(Exts((self.0 .0.try_done(field).ok(),)))
    }
}

// Todo: we might want to also implement this for higher arity. maybe add a impl_request_exts-style macro for it.
impl<'xml, A: FromXml<'xml>, B: FromXml<'xml>> FromXml<'xml> for Exts<(Option<A>, Option<B>)> {
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        A::matches(id, field) || B::matches(id, field)
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        let id = deserializer.parent();
        if A::matches(id, None) {
            A::deserialize(&mut into.0 .0, field, deserializer)?;
        } else if B::matches(id, None) {
            B::deserialize(&mut into.0 .1, field, deserializer)?;
        } else {
            deserializer.ignore()?;
        }
        Ok(())
    }

    type Accumulator = ExtsAcc<(A::Accumulator, B::Accumulator)>;
    const KIND: Kind = Kind::Element;
}

impl<'xml, A: FromXml<'xml>, B: FromXml<'xml>> Accumulate<Exts<(Option<A>, Option<B>)>>
    for ExtsAcc<(A::Accumulator, B::Accumulator)>
{
    fn try_done(self, field: &'static str) -> Result<Exts<(Option<A>, Option<B>)>, Error> {
        let (a, b) = self.0;
        Ok(Exts((a.try_done(field).ok(), b.try_done(field).ok())))
    }
}

impl<T: ExtTuple> Exts<T> {
    /// Borrow the response extension of type `X`, if the server returned it.
    ///
    /// ```ignore
    /// let rgp = rsp.extension().and_then(Exts::get::<RgpInfData>);
    /// ```
    pub fn get<X: 'static>(&self) -> Option<&X> {
        self.0.find::<X>()
    }
}

/// A tuple of optional response extensions supporting type-directed lookup.
///
/// Implemented for tuples up to arity 3. Used by [`Exts::get`]; callers do not
/// interact with it directly.
pub trait ExtTuple {
    /// Borrow the contained extension of type `X`, if present.
    fn find<X: 'static>(&self) -> Option<&X>;
}

impl ExtTuple for () {
    fn find<X: 'static>(&self) -> Option<&X> {
        None
    }
}

macro_rules! impl_ext_tuple {
    ($($T:ident . $idx:tt),+) => {
        impl<$($T: 'static),+> ExtTuple for ($(Option<$T>,)+) {
            fn find<X: 'static>(&self) -> Option<&X> {
                $(
                    if let Some(found) = self.$idx.as_ref().and_then(|v| (v as &dyn Any).downcast_ref::<X>()) {
                        return Some(found);
                    }
                )+
                None
            }
        }
    };
}

impl_ext_tuple!(A.0);
impl_ext_tuple!(A.0, B.1);
impl_ext_tuple!(A.0, B.1, C.2);

impl<D, T: ExtTuple> Response<D, Exts<T>> {
    /// Borrow the response extension of type `X`, if the server returned it.
    pub fn ext<X: 'static>(&self) -> Option<&X> {
        self.extension().and_then(|exts| exts.get::<X>())
    }

    /// Extract several response extensions in one destructure, axum-extractor style.
    ///
    /// ```ignore
    /// let (rgp, fee): (&RgpInfData, Option<&FeeData>) = rsp.extract()?;
    /// ```
    pub fn extract<'a, E: FromExts<'a>>(&'a self) -> Result<E, MissingExtension> {
        E::from_exts(self.extension())
    }
}

/// Error returned by [`Response::extract`] when a *required* extension (a bare
/// `&X` in the extracted tuple) is not present in the response.
#[derive(Debug)]
pub struct MissingExtension(pub &'static str);

/// Extract typed values from a response's `<extension>` set (axum-extractor style).
///
/// Implemented for `&X` (required), `Option<&X>` (optional), and tuples of those
/// so several extensions can be pulled in a single [`Response::extract`].
pub trait FromExts<'a>: Sized {
    /// Pull `Self` out of the (possibly absent) extension set.
    fn from_exts<T: ExtTuple>(exts: Option<&'a Exts<T>>) -> Result<Self, MissingExtension>;
}

impl<'a, X: 'static> FromExts<'a> for &'a X {
    fn from_exts<T: ExtTuple>(exts: Option<&'a Exts<T>>) -> Result<Self, MissingExtension> {
        exts.and_then(|e| e.get::<X>())
            // Todo should we instead use the XML namespace here?
            .ok_or(MissingExtension(std::any::type_name::<X>()))
    }
}

impl<'a, X: 'static> FromExts<'a> for Option<&'a X> {
    fn from_exts<T: ExtTuple>(exts: Option<&'a Exts<T>>) -> Result<Self, MissingExtension> {
        Ok(exts.and_then(|e| e.get::<X>()))
    }
}

macro_rules! impl_from_exts_tuple {
    ($($G:ident),+) => {
        impl<'a, $($G: FromExts<'a>),+> FromExts<'a> for ($($G,)+) {
            fn from_exts<T: ExtTuple>(exts: Option<&'a Exts<T>>) -> Result<Self, MissingExtension> {
                Ok(($($G::from_exts(exts)?,)+))
            }
        }
    };
}

impl_from_exts_tuple!(A);
impl_from_exts_tuple!(A, B);
impl_from_exts_tuple!(A, B, C);

#[cfg(test)]
mod tests {
    use super::{Exts, Profile, ProfiledCommand};
    use crate::domain::info::{DomainInfo, InfoData};
    use crate::domain::update::{DomainChangeInfo, DomainUpdate};
    use crate::extensions::rgp::request::{
        RgpRequestInfoResponse, RgpRequestUpdateResponse, RgpRestoreRequest, Update,
    };
    use crate::extensions::rgp::RgpStatus;
    use crate::response::Response;
    use crate::tests::{get_xml, CLTRID};
    use crate::xml;

    /// Example registry profile.
    struct Verisign;

    // (domain, info) with no request extension still yields the RGP `infData`
    // that the server volunteers
    impl Profile<DomainInfo<'_>, ()> for Verisign {
        type Response = InfoData;
        type RespExts = Exts<(Option<RgpRequestInfoResponse>,)>;
    }

    #[test]
    fn request_tuple_matches_legacy_serialization() {
        // Build the same RGP restore request as the legacy test, but through the
        // `RequestExts` tuple path, and assert byte-identical output.
        let mut command = DomainUpdate::new("eppdev.com");
        command.info(DomainChangeInfo {
            registrant: None,
            auth_info: None,
        });
        let exts = (Update {
            data: RgpRestoreRequest::default(),
        },);

        let document = ProfiledCommand {
            command: &command,
            exts: &exts,
            client_tr_id: CLTRID,
        };

        let expected = get_xml("request/extensions/rgp_restore_request.xml").unwrap();
        similar_asserts::assert_eq!(expected, xml::serialize(&document).unwrap());
    }

    #[test]
    fn profile_response_only_rgp() {
        // Deserialize through the profile-determined types: resData -> InfoData,
        // extension -> Exts<(Option<RgpRequestInfoResponse>,)>.
        type Resp = Response<
            <Verisign as Profile<DomainInfo<'static>, ()>>::Response,
            <Verisign as Profile<DomainInfo<'static>, ()>>::RespExts,
        >;

        let xml = get_xml("response/extensions/domain_info_rgp.xml").unwrap();
        let rsp = xml::deserialize::<Resp>(&xml).unwrap();

        assert_eq!(rsp.res_data().unwrap().name, "eppdev-1.com");
        let rgp = rsp
            .ext::<RgpRequestInfoResponse>()
            .expect("rgp infData present");
        assert_eq!(rgp.rgp_status.len(), 2);
        assert_eq!(rgp.rgp_status[0], RgpStatus::AddPeriod);
        assert_eq!(rgp.rgp_status[1], RgpStatus::RenewPeriod);
    }

    #[test]
    fn exts_product_of_options_routes_by_element() {
        // Two declared response extensions; only `rgp:infData` is in the fixture,
        // so the `rgp:upData` slot must come back `None`.
        type E = Exts<(
            Option<RgpRequestInfoResponse>,
            Option<RgpRequestUpdateResponse>,
        )>;

        let xml = get_xml("response/extensions/domain_info_rgp.xml").unwrap();
        let rsp = xml::deserialize::<Response<InfoData, E>>(&xml).unwrap();

        assert!(
            rsp.ext::<RgpRequestInfoResponse>().is_some(),
            "infData present"
        );
        assert!(
            rsp.ext::<RgpRequestUpdateResponse>().is_none(),
            "upData absent"
        );
    }

    #[test]
    fn extract_required_and_optional() {
        // axum-style: `&X` is required, `Option<&X>` is optional, pulled in one
        // destructure with the types annotated at the call site.
        type E = Exts<(
            Option<RgpRequestInfoResponse>,
            Option<RgpRequestUpdateResponse>,
        )>;

        let xml = get_xml("response/extensions/domain_info_rgp.xml").unwrap();
        let rsp = xml::deserialize::<Response<InfoData, E>>(&xml).unwrap();

        let (rgp, upd): (&RgpRequestInfoResponse, Option<&RgpRequestUpdateResponse>) =
            rsp.extract().unwrap();
        assert_eq!(rgp.rgp_status.len(), 2);
        assert!(upd.is_none());

        // A required extension the server did not return is an error, not a panic.
        let err = rsp.extract::<&RgpRequestUpdateResponse>().unwrap_err();
        assert!(err.0.contains("RgpRequestUpdateResponse"));
    }
}
