//! Mapping for the Registry Grace Period extension
//!
//! As described in [RFC 3915](https://tools.ietf.org/html/rfc3915).

pub mod poll; // Technically a separate extension (different namespace, RFC)
pub mod report;
pub mod request;

pub const XMLNS: &str = "urn:ietf:params:xml:ns:rgp-1.0";
