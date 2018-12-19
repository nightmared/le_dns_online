use std::fmt;
use serde::{Deserialize, Deserializer, de::Visitor};
use serde_derive::*;
use std::time::SystemTime;

use crate::net::*;
use crate::config::*;

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

// So trivial, right ! (actually, this is a rather convolved way of doing somethign simple)
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

/// Get the list of all available domains pertaining to this user
pub fn query_available_domains<'a>(api_key: &'a str) -> Result<Vec<Domain<'a>>, Error> {
    let data: Vec<Domain<'a>> = execute_query(&api_key, "/domain/", get_data, parse_json)?;
    Ok(
        data
        .into_iter()
        .map(|mut x| {
            // Let's not forget to add the proper API key to each and every one of theses little domains
            x.api_key = api_key;
            x
        })
        .collect()
    )
}

impl<'a> Domain<'a> {
    pub fn add_version(&self, name: &str) -> Result<Version, Error> {
        let domain_version_url = format!("/domain/{}/version", self.name);
        let domain_version_post_data = vec![PostData("name", &name)];
        execute_query(self.api_key, &domain_version_url, post_data(&domain_version_post_data), parse_json)
    }

    // Append a record to the zone corresponding to destination
    pub fn append_record(&self, destination: &Version, record: &Record) -> Result<Record, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let ttl = record.ttl.as_string();
        let post_entries = vec![PostData("name", &record.name), PostData("type", &record.record_type), PostData("priority", "12"), PostData("ttl", &ttl), PostData("data", &record.data)];
        execute_query(self.api_key, &dest_zone_url, post_data(&post_entries), parse_json)
    }

    // copy all the records from source to destination and return the new zone records
    pub fn copy_zone(&self, source: Vec<Record>, destination: &Version) -> Result<Vec<Record>, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let mut dest_zone: Vec<Record> = execute_query(self.api_key, &dest_zone_url, get_data, parse_json)?;
        for ref entry in source {
            dest_zone.push(self.append_record(destination, entry)?);
        }
        Ok(dest_zone)
    }

    pub fn enable_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/enable", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, patch_data, |_| -> Result<(), Error> { Ok(()) })
    }

    pub fn delete_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, delete_data, |_| -> Result<(), Error> { Ok(()) })
    }

    pub fn get_versions(&self) -> Result<Vec<Version>, Error> {
        let url = format!("/domain/{}/version", self.name);
        execute_query(self.api_key, &url, get_data, parse_json)
    }

    pub fn get_current_zone(&self) -> Result<Vec<Record>, Error> {
        let zone_url = format!("/domain/{}/zone", self.name);
        execute_query(self.api_key, &zone_url, get_data, parse_json)
    }

    pub fn add_record(self, subrecord: &str, txt_value: &str) -> Result<(), Error> {
        let zone = self.get_current_zone()?;

        // create a new version
        let new_version = self.add_version(&format!("tmp-{:?}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()))?;
        self.copy_zone(zone, &new_version)?;

        // add the record needed by Let's Encrypt
        self.append_record(&new_version,
            &Record {
                name: subrecord.into(),
                record_type: "TXT".into(),
                ttl: TTL { val: 86400 },
                data: txt_value.into()
            })?;

        // write to the zone file the id of the previous active zone, to be able to rollback later
        let current_version = self.get_versions()?.into_iter().filter(|x| x.active).next().unwrap();
        self.append_record(&new_version,
            &Record {
                name: format!("original_zone_id.{}", subrecord),
                record_type: "TXT".into(),
                ttl: TTL { val: 86400 },
                data: current_version.uuid_ref
            })?;

        // make the new version the active one
        self.enable_version(&new_version)?;

        Ok(())
    }

    pub fn delete_record(self, subrecord: &str, _txt_value: &str) -> Result<(), Error> {
        let matching_record = format!("original_zone_id.{}", subrecord);
        let zone_ref = match self.get_current_zone()?.into_iter().filter(|x| x.name == matching_record).next() {
            Some(x) => x,
            None => return Err(Error::NoRecord)
        };
        let versions = self.get_versions()?;
        let tmp_version = versions.iter().filter(|x| x.active).next().unwrap();
        let original_version = match versions.iter().filter(|x| x.uuid_ref == zone_ref.data).next() {
            Some(x) => x,
            None => return Err(Error::InvalidVersion)
        };
        self.enable_version(&original_version)?;
        self.delete_version(&tmp_version)?;

        Ok(())

    }
}