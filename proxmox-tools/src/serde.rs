//! Serialization helpers for serde

/// Serialize DateTime<Local> as RFC3339.
///
/// Usage example:
/// ```
/// # pub extern crate proxmox_tools;
/// # mod proxmox { pub use proxmox_tools as tools; }
///
/// use chrono::{DateTime, TimeZone, Utc};
/// use serde::{Deserialize, Serialize};
///
/// # #[derive(Debug)]
/// #[derive(Deserialize, PartialEq, Serialize)]
/// struct Foo {
///     #[serde(with = "proxmox::tools::serde::date_time_as_rfc3339")]
///     date: DateTime<Utc>,
/// }
///
/// let obj = Foo { date: Utc.timestamp_millis(86400000) }; // random test value
/// let json = serde_json::to_string(&obj).unwrap();
/// assert_eq!(json, r#"{"date":"1970-01-02T00:00:00+00:00"}"#);
///
/// let deserialized: Foo = serde_json::from_str(&json).unwrap();
/// assert_eq!(obj, deserialized);
/// ```
pub mod date_time_as_rfc3339 {
    use chrono::{DateTime, TimeZone};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, Tz>(time: &DateTime<Tz>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        Tz: TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        serializer.serialize_str(&time.to_rfc3339())
    }

    pub fn deserialize<'de, D, Tz>(deserializer: D) -> Result<DateTime<Tz>, D::Error>
    where
        D: Deserializer<'de>,
        Tz: TimeZone,
        DateTime<Tz>: std::str::FromStr,
        <DateTime<Tz> as std::str::FromStr>::Err: std::string::ToString,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            string
                .parse::<DateTime<Tz>>()
                .map_err(|err| Error::custom(err.to_string()))
        })
    }
}

/// Serialize Vec<u8> as base64 encoded string.
pub mod bytes_as_base64 {

    use base64;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(data.as_ref()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
        })
    }
}
