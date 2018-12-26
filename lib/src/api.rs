use std::fmt;
use serde::{Deserialize, Deserializer, de::Visitor};
use serde_derive::*;

use crate::net::*;
use crate::error::Error;

// We need to implement this special type because the API can return the tty both as a number and
// as a string !? As a result, we need to parse this entry properly
#[derive(Debug)]
pub struct TTL {
    pub val: usize
}

impl TTL {
    pub fn as_string(&self) -> String {
        format!("{}", self.val)
    }
}

// So trivial, right ! (actually, this is a rather convolved way of doing something simple)
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

/// A DNS domain.
/// For API design reason, we also store the API key inside the domain.
#[derive(Deserialize, Debug)]
pub struct Domain<'a> {
    #[serde(skip)]
    pub api_key: &'a str,
    pub id: usize,
    pub name: String,
    pub dnssec: bool,
    pub external: bool
}

/// A DNS entry.
/// The query type is stored as a string ("TXT", "AAAA", ...).
#[derive(Deserialize, Debug)]
pub struct Record {
    pub id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub ttl: TTL,
    pub data: String
}

/// A DNS Zone.
/// Please note this zone may not be the one currently active for the domain.
#[derive(Deserialize, Debug)]
pub struct Version {
    pub uuid_ref: String,
    pub name: String,
    pub active: bool
}

/// Get the list of all available domains pertaining to this user.
pub fn query_available_domains<'a>(api_key: &'a str) -> Result<Vec<Domain<'a>>, Error> {
    let data: Vec<Domain<'a>> = execute_query(&api_key, "/domain/", get_data, parse_json)?;
    Ok(
        data
        .into_iter()
        .map(|mut x| {
            // Let's not forget to add the proper API key to each and every one of theses cute little domains
            x.api_key = api_key;
            x
        })
        .collect()
    )
}

impl<'a> Domain<'a> {
    /// Try to extract the longest matching domain from the list of our available domains and the internal part of the name.
    /// e.g. extract_domain("this.is.a.dummy.test.fr.", {Domain("test.fr"), Domain("nope.fr")}) should return
    /// the domain associated with "test.fr". and the internal path, aka "this.is.a.dummy."
    pub fn find_and_extract_path(full_domain_name: &'a str, domains: Vec<Domain<'a>>) -> Option<(Self, &'a str)> {
        let mut full_domain_name = full_domain_name;
        // delete any trailing dot
        if full_domain_name.ends_with(".") {
            full_domain_name = &full_domain_name[0..full_domain_name.len()-1];
        }
        for available_domain in domains {
            if full_domain_name.ends_with(&available_domain.name) {
                let max_len = full_domain_name.len()-available_domain.name.len()-1;
                return Some((available_domain, &full_domain_name[0..max_len]));
            }
        }
        None
    }

    /// Create a new (disabled at the moment) zone.
    pub fn add_version(&self, name: &str) -> Result<Version, Error> {
        let domain_version_url = format!("/domain/{}/version", self.name);
        let domain_version_post_data = vec![PostData("name", &name)];
        execute_query(self.api_key, &domain_version_url, post_data(&domain_version_post_data), parse_json)
    }

    /// Extract all record with a name of "entry_name" and with a value of "entry_value" (or any value if entry_value is None) from the zone 'zone'.
    pub fn get_record(&self, zone: &Version, entry_name: &str, entry_value: Option<&str>) -> Result<Option<Vec<Record>>, Error> {
        let entries = self.get_zone_records(zone)?;
        let mut res = vec![];
        for e in entries {
            if e.name == entry_name {
                if let Some(data) = entry_value {
                    if data != e.data {
                        continue;
                    }
                }
                res.push(e);
            }
        }
        if res.len() > 0 {
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }

    /// Append a new entry 'record' to the zone 'destination'.
    pub fn append_record(&self, destination: &Version, record: &Record) -> Result<Record, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let ttl = record.ttl.as_string();
        let post_entries = vec![PostData("name", &record.name), PostData("type", &record.record_type), PostData("priority", "12"), PostData("ttl", &ttl), PostData("data", &record.data)];
        execute_query(self.api_key, &dest_zone_url, post_data(&post_entries), parse_json)
    }

    /// Copy all the records from 'source' to the zone 'destination' and return the updated zone records.
    /// This will not erase the curretn entries but append next to the them.
    pub fn copy_zone(&self, source: Vec<Record>, destination: &Version) -> Result<Vec<Record>, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let mut dest_zone: Vec<Record> = execute_query(self.api_key, &dest_zone_url, get_data, parse_json)?;
        for ref entry in source {
            dest_zone.push(self.append_record(destination, entry)?);
        }
        Ok(dest_zone)
    }

    /// Enable a specific zone as the current one for the domain.
    pub fn enable_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/enable", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, patch_data, |_| -> Result<(), Error> { Ok(()) })
    }

    /// Delete an old zone.
    /// As a result, deleting the current zone will fail.
    pub fn delete_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, delete_data, |_| -> Result<(), Error> { Ok(()) })
    }

    /// Return the list of all available zones.
    pub fn get_versions(&self) -> Result<Vec<Version>, Error> {
        let url = format!("/domain/{}/version", self.name);
        execute_query(self.api_key, &url, get_data, parse_json)
    }

    /// Retrieve the Version describing the currently enable zone
    pub fn get_current_zone(&self) -> Result<Version, Error> {
        let url = format!("/domain/{}/version", self.name);
        let versions: Vec<Version> = execute_query(self.api_key, &url, get_data, parse_json)?;
        Ok(
            versions
            .into_iter()
            .filter(|x| x.active)
            .next()?
        )
    }

    /// Return the list of all the records in the zone 'zone'.
    pub fn get_zone_records(&self, zone: &Version) -> Result<Vec<Record>, Error> {
        let zone_url = format!("/domain/{}/version/{}/zone", self.name, zone.uuid_ref);
        execute_query(self.api_key, &zone_url, get_data, parse_json)
    }

    /// Add a new record to the zone "destination".
    pub fn add_record(&self, destination: &Version, entry_name: impl Into<String>, entry_type: impl Into<String>,
    entry_value: impl Into<String>, entry_ttl: usize) -> Result<Record, Error> {
        Ok(self.append_record(destination,
            &Record {
                // The id doesn't actually matter, it isn't passed on to the online.net API
                id: 0,
                name: entry_name.into(),
                record_type: entry_type.into(),
                ttl: TTL { val: entry_ttl },
                data: entry_value.into()
            })?
        )
    }

    /// Delete a record in 'zone' matching 'record'
    pub fn delete_record(&self, zone: &Version, record: &Record) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/zone/{}", self.name, zone.uuid_ref, record.id);
        execute_query(self.api_key, &url, delete_data, throw_value)?;
        Ok(())
    }
}