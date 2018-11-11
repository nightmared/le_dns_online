use std::fmt;
use serde::{Deserialize, Deserializer, de::Visitor};

#[derive(Deserialize, Debug)]
pub struct Domain<'a> {
    #[serde(skip)]
    pub api_key: &'a str,
    pub id: usize,
    pub name: String,
    pub dnssec: bool,
    pub external: bool
}

// We need to implement this special type because the API can return the tty both as a number and
// as a string !?
#[derive(Debug)]
pub struct TTL {
    pub val: usize
}

impl TTL {
    pub fn as_string(&self) -> String {
        format!("{}", self.val)
    }
}

impl<'de> Deserialize<'de> for TTL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct UsizeVisitor;
        impl<'de> Visitor<'de> for UsizeVisitor {
            type Value = TTL;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                fmt.write_str("usize compatible type")
            }

            fn visit_u32<E>(self, val: u32) -> Result<Self::Value, E> {
                Ok(TTL {
                    val: val as usize
                })
            }

            fn visit_u64<E>(self, val: u64) -> Result<Self::Value, E> {
                Ok(TTL {
                    val: val as usize
                })
            }

            fn visit_str<E>(self, val: &str) -> Result<Self::Value, E> {
                Ok(TTL {
                    val: val.parse().unwrap()
                })
            }

        }
        deserializer.deserialize_any(UsizeVisitor)

    }
}

#[derive(Deserialize, Debug)]
pub struct Record {
    //id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub ttl: TTL,
    pub data: String
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub uuid_ref: String,
    pub name: String,
    pub active: bool
}
