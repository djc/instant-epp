//! XSD Duration format

use std::ops::Div;
use std::str::FromStr;

use instant_xml::{FromXml, ToXml};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct XsdDuration {
    months: i64,
    seconds: f64,
}

impl XsdDuration {
    pub fn new(months: i64, seconds: f64) -> Result<Self, InvalidMinutesOrSeconds> {
        if months < 0 && seconds > 0.0 || months > 0 && seconds < 0.0 {
            return Err(InvalidMinutesOrSeconds);
        }
        Ok(Self { months, seconds })
    }

    fn is_zero(&self) -> bool {
        self.months == 0 && self.seconds == 0.0
    }
}

impl FromStr for XsdDuration {
    type Err = ParseError;

    /// Parses an XSD duration string into [`XsdDuration`].
    ///
    /// Algorithm based on https://www.w3.org/TR/xmlschema11-2/#f-durationMap
    ///
    /// DUR consists of possibly a leading '-', followed by 'P' and then an instance Y of duYearMonthFrag and/or an instance D of duDayTimeFrag:
    /// Return a duration whose:
    /// * months value is 12 * Y + M
    /// * seconds value is 86400 * D + (3600 * H + 60 * M + S)
    ///
    /// where Y, M, D, H, M, S are the values parsed from the string.
    ///
    /// All values default to 0 if not present.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // DUR consists of possibly a leading '-', followed by 'P' and then an instance Y of duYearMonthFrag and/or an instance D of duDayTimeFrag:
        let sgn = if s.starts_with('-') { -1 } else { 1 };
        let s = s.trim_start_matches('-');
        if !s.starts_with('P') {
            return Err(ParseError);
        }
        let s = &s[1..];

        // duYearMonthFrag
        let (s, months) = {
            let mut y = 0;
            let s = match s.split_once("Y") {
                Some((l, r)) => {
                    y = l.parse::<u32>().map_err(|_| ParseError)?;
                    r
                }
                None => s,
            };
            let mut m = 0;
            let s = match s.split_once("M") {
                // skip months if the M was part of duDayTimeFrag
                Some((l, r)) if !l.contains('T') => {
                    m = l.parse::<u32>().map_err(|_| ParseError)?;
                    r
                }
                _ => s,
            };

            (s, 12 * (y as i64) + (m as i64) * sgn)
        };

        // duDayTimeFrag
        let seconds = {
            let mut d = 0;

            let s = match s.split_once('D') {
                Some((l, r)) => {
                    d = l.parse::<u32>().map_err(|_| ParseError)?;
                    r
                }
                None => s,
            };
            if !s.starts_with("T") {
                return Ok(Self {
                    months,
                    seconds: (86400 * d) as f64,
                });
            }
            let s = &s[1..];
            let t = {
                let mut h = 0;
                let mut m = 0;
                let mut ss = 0.0;
                let s = match s.split_once('H') {
                    Some((l, r)) => {
                        h = l.parse::<u32>().map_err(|_| ParseError)?;
                        r
                    }
                    None => s,
                };
                let s = match s.split_once('M') {
                    Some((l, r)) => {
                        m = l.parse::<u32>().map_err(|_| ParseError)?;
                        r
                    }
                    None => s,
                };
                if let Some((l, _r)) = s.split_once('S') {
                    ss = l.parse::<f64>().map_err(|_| ParseError)?;
                }

                (3600 * h) as f64 + (60 * m) as f64 + ss
            };

            (86400 * d) as f64 + t
        };

        Ok(Self { months, seconds })
    }
}

impl<'xml> FromXml<'xml> for XsdDuration {
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
        if into.is_some() {
            return Err(instant_xml::Error::DuplicateValue(field));
        }

        if let Some(value) = deserializer.take_str()? {
            let duration = value.parse().map_err(|_| {
                instant_xml::Error::Other(format!("failed to parse xsd duration: {}", value))
            })?;
            *into = Some(duration);
        }

        Ok(())
    }

    type Accumulator = Option<Self>;

    const KIND: instant_xml::Kind = instant_xml::Kind::Scalar;
}

impl ToXml for XsdDuration {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        _field: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        serializer.write_str(&format_duration_inner(self))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct InvalidMinutesOrSeconds;

impl std::error::Error for InvalidMinutesOrSeconds {}

impl std::fmt::Display for InvalidMinutesOrSeconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid minutes or seconds value")
    }
}

#[derive(Debug)]
pub struct ParseError;

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse xsd duration")
    }
}

/// Serialize duration to XML duration string
///
/// See https://www.w3.org/TR/xmlschema11-2/#duration
pub fn format_duration<D>(duration: D) -> Result<String, D::Error>
where
    D: TryInto<XsdDuration>,
{
    let duration: XsdDuration = duration.try_into()?;
    Ok(format_duration_inner(&duration))
}
/// Serialize duration to XML duration string
///
/// https://www.w3.org/TR/xmlschema11-2/#f-durationCanMap
fn format_duration_inner(duration: &XsdDuration) -> String {
    if duration.is_zero() {
        return "P0D".to_owned();
    }

    let mut buf = if duration.months < 0 || duration.seconds < 0.0 {
        String::from("-P")
    } else {
        String::from("P")
    };
    // https://www.w3.org/TR/xmlschema11-2/#f-duYMCan
    let years = (duration.months / 12) as u64;
    let months = (duration.months % 12) as u64;

    if years > 0 {
        buf.push_str(&format!("{}Y", years));
    }
    if months > 0 {
        buf.push_str(&format!("{}M", months));
    }
    // https://www.w3.org/TR/xmlschema11-2/#f-duDTCan
    if duration.seconds == 0.0 {
        return buf;
    }

    let days = (duration.seconds.div_euclid(86400.0)) as u64;
    let hours = ((duration.seconds % 86400.0).div_euclid(3600.0)) as u64;
    let minutes = ((duration.seconds % 3600.0).div(60.0)) as u64;
    let seconds = duration.seconds % 60.0;

    if duration.seconds != 0.0 {
        if days > 0 {
            buf.push_str(&format!("{}D", days));
        }

        if hours == 0 && minutes == 0 && seconds == 0.0 {
            return buf;
        }

        buf.push('T');

        if hours > 0 {
            buf.push_str(&format!("{}H", hours));
        }
        if minutes > 0 {
            buf.push_str(&format!("{}M", minutes));
        }
        if seconds > 0.0 {
            if seconds.fract() > 0.0 {
                buf.push_str(&format!("{:.4}S", seconds));
            } else {
                buf.push_str(&format!("{}S", seconds.trunc() as u64));
            }
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86400;

    #[test]
    fn construct() {
        let _ = XsdDuration::new(12, 3600.0).unwrap();
        let _ = XsdDuration::new(-12, -3600.0).unwrap();
        assert!(XsdDuration::new(12, -3600.0).is_err());
        assert!(XsdDuration::new(-12, 3600.0).is_err());
    }

    #[test]
    fn ser() {
        let dur = XsdDuration::new(0, (3600 + 60 + 1) as f64).unwrap(); // 1 hour, 1 minute, 1 second
        let s = format_duration(dur).unwrap();
        assert_eq!(s, "PT1H1M1S");

        let dur = XsdDuration::new(0, 0.0).unwrap();
        let s = format_duration(dur).unwrap();
        assert_eq!(s, "P0D");

        // This is totally flawed but the spec demands it
        let dur = XsdDuration::new(13, (DAY + 3600 + 60 + 1) as f64).unwrap(); // 1 year, 1 month, 1 day, 1 hour, 1 minute, 1 second
        let s = format_duration(dur).unwrap();
        assert_eq!(s, "P1Y1M1DT1H1M1S");

        let dur = XsdDuration::new(13, (5 * DAY) as f64).unwrap(); // 1 year, 1 month, 5 days
        let s = format_duration(dur).unwrap();
        assert_eq!(s, "P1Y1M5D");
    }

    #[test]
    fn deser() {
        let s = "PT1H1M1S";
        let dur: XsdDuration = s.parse().unwrap();
        assert_eq!(dur, XsdDuration::new(0, (3600 + 60 + 1) as f64).unwrap());

        let s = "P0D";
        let dur: XsdDuration = s.parse().unwrap();
        assert_eq!(dur, XsdDuration::new(0, 0.0).unwrap());
        let s = "P1Y1M1DT1H1M1S";
        let dur: XsdDuration = s.parse().unwrap();
        assert_eq!(
            dur,
            XsdDuration::new(12 * 30, (30 * DAY + DAY + 3600 + 60 + 1) as f64).unwrap()
        );

        let s = "P1Y1M5DT0H0M0S";
        let dur: XsdDuration = s.parse().unwrap();
        assert_eq!(
            dur,
            XsdDuration::new(12 * 30, (30 * DAY + 5 * DAY) as f64).unwrap()
        );
    }
}
