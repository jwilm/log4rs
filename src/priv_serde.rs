use std::fmt;
use serde::de::{self, Unexpected};
use log::LogLevelFilter;
use std::time::Duration;

#[derive(PartialEq, Debug)]
pub struct DeLogLevelFilter(pub LogLevelFilter);

impl de::Deserialize for DeLogLevelFilter {
    fn deserialize<D>(d: D) -> Result<DeLogLevelFilter, D::Error>
        where D: de::Deserializer
    {
        struct V;

        impl de::Visitor for V {
            type Value = DeLogLevelFilter;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("string")
            }

            fn visit_str<E>(self, v: &str) -> Result<DeLogLevelFilter, E>
                where E: de::Error
            {
                v.parse().map(DeLogLevelFilter).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        d.deserialize_str(V)
    }
}

#[derive(PartialEq, Debug)]
pub struct DeDuration(pub Duration);

impl de::Deserialize for DeDuration {
    fn deserialize<D>(d: D) -> Result<DeDuration, D::Error>
        where D: de::Deserializer
    {
        u64::deserialize(d).map(|r| DeDuration(Duration::from_secs(r)))
    }
}
