use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Default, Debug)]
pub struct TimeStamp(DateTime<FixedOffset>);

impl Deref for TimeStamp {
    type Target = DateTime<FixedOffset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y/%m/%d %H:%M:%S"))
    }
}

impl Serialize for TimeStamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
