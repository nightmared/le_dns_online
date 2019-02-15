use std::fmt;
use serde::Deserializer;
use serde::de::Visitor;
use serde_derive::*;

use crate::net::*;
use crate::error::Error;
use crate::bind::to_bind;

// So trivial, right ! (actually, this is a rather convolved way of doing something simple)
// This artefact is solely necessary as a byproduct of some tiny issues in the API. Indeed,
// the server can return the ttl both as a number and as a string.
fn deserialize_ttl<'de, D>(deserializer: D) -> Result<usize, D::Error>
where D: Deserializer<'de> {
	struct UsizeVisitor;
	impl<'de> Visitor<'de> for UsizeVisitor {
		type Value = usize;

		fn expecting(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
			fmt.write_str("usize compatible type")
		}

		fn visit_u32<E>(self, val: u32) -> Result<Self::Value, E> {
			Ok(val as usize)
		}

		fn visit_u64<E>(self, val: u64) -> Result<Self::Value, E> {
			Ok(val as usize)
		}

		fn visit_str<E>(self, val: &str) -> Result<Self::Value, E> {
			Ok(val.parse().unwrap())
		}
	}
	deserializer.deserialize_any(UsizeVisitor)

}

/// A DNS domain.
/// For API design reasons, we also store the API key inside the domain.
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
#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    pub id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: DNSType,
	#[serde(deserialize_with = "deserialize_ttl")]
    pub ttl: usize,
    pub data: String
}

impl Record {
    pub fn new(entry_name: impl Into<String>, entry_type: impl Into<DNSType>,
        entry_value: impl Into<String>, entry_ttl: usize) -> Record {
            Record {
                // The id doesn't actually matter, it isn't passed on to the online.net API
                id: 0,
                name: entry_name.into(),
                record_type: entry_type.into(),
                ttl: entry_ttl,
                data: entry_value.into()
            }
    }
}


/// A DNS Zone.
/// Please keep in mind that this zone may not be the one currently active for the domain.
#[derive(Deserialize, Debug)]
pub struct Version {
    #[serde(rename = "uuid_ref")]
    pub uuid: String,
    pub name: String,
    pub active: bool
}

/// Get the list of all available domains pertaining to this user.
pub fn query_available_domains<'a>(api_key: &'a str) -> Result<Vec<Domain<'a>>, Error> {
    let data: Vec<Domain<'a>> = execute_query(&api_key, "/domain/", query_set_type(HTTPOp::GET), parse_json)?;
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

    /// Extract all records with a name of "entry_name" and with a value of "entry_value" (or any value if entry_value is None) from the zone 'zone'.
    pub fn filter_records(&self, zone: &Version, entry_name: &str, entry_value: Option<&str>) -> Result<Option<Vec<Record>>, Error> {
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
    /// The target zone MUST be inactive.
    pub fn add_record(&self, destination: &Version, record: &Record) -> Result<Record, Error> {
        let dst = self.get_version(&destination.uuid)?;
        if dst.active {
            return Err(Error::ActiveZoneForbidden);
        }

        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, dst.uuid);
        let ttl = record.ttl.to_string();
        let record_type = String::from(&record.record_type);
        let post_entries = vec![FormData("name", &record.name), FormData("type", &record_type), FormData("priority", "12"), FormData("ttl", &ttl), FormData("data", &record.data)];
        execute_query(self.api_key, &dest_zone_url, query_set_type(HTTPOp::POST(&post_entries)), parse_json)
    }

    /// Copy all the records from 'source' to the zone 'destination' and return the updated zone records.
    /// This will not erase the current entries but append next to the them.
    pub fn copy_records(&self, source: Vec<Record>, destination: &Version) -> Result<Vec<Record>, Error> {
        let dst = self.get_version(&destination.uuid)?;
        if dst.active {
            return Err(Error::ActiveZoneForbidden);
        }

        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, dst.uuid);
        let mut dest_zone: Vec<Record> = execute_query(self.api_key, &dest_zone_url, query_set_type(HTTPOp::GET), parse_json)?;
        for ref entry in source {
            dest_zone.push(self.add_record(destination, entry)?);
        }
        Ok(dest_zone)
    }

    /// Populate the zone "destination" with 'records'.
    /// Note this will destroy any prior entry in that zone.
    /// Internally this calls the endpoint 
    /// /domain/{domain_name}/version/{version_id}/zone_from_bind
    pub fn set_zone_entries(&self, destination: &Version, records: &[Record]) -> Result<(), Error> {
        let dst = self.get_version(&destination.uuid)?;
        if dst.active {
            return Err(Error::ActiveZoneForbidden);
        }

        let bind_zone = to_bind(records);

        let domain_version_url = format!("/domain/{}/version/{}/zone_from_bind", self.name, dst.uuid);
        execute_query(self.api_key, &domain_version_url, query_set_type(HTTPOp::PUT(&bind_zone)), throw_value)
    }

    /// Create a new (disabled at the moment) zone.
    pub fn add_version(&self, name: &str) -> Result<Version, Error> {
        let domain_version_url = format!("/domain/{}/version", self.name);
        let domain_version_post_data = vec![FormData("name", &name)];
        execute_query(self.api_key, &domain_version_url, query_set_type(HTTPOp::POST(&domain_version_post_data)), parse_json)
    }

    /// Enable a specific zone as the current one for the domain.
    pub fn enable_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/enable", self.name, v.uuid);
        execute_query(self.api_key, &url, query_set_type(HTTPOp::PATCH(None)), throw_value)
    }

    /// Delete an old zone.
    /// As a result, deleting the current zone will fail.
    pub fn delete_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}", self.name, v.uuid);
        execute_query(self.api_key, &url, query_set_type(HTTPOp::DELETE), |_| -> Result<(), Error> { Ok(()) })
    }

	/// Return the version of a given uuid
	pub fn get_version(&self, uuid: &str) -> Result<Version, Error> {
        let url = format!("/domain/{}/version/{}", self.name, uuid);
        execute_query(self.api_key, &url, query_set_type(HTTPOp::GET), parse_json)
}

    /// Return the list of all available zones.
    pub fn get_versions(&self) -> Result<Vec<Version>, Error> {
        let url = format!("/domain/{}/version", self.name);
        execute_query(self.api_key, &url, query_set_type(HTTPOp::GET), parse_json)
    }

    /// Retrieve the Version describing the currently enable zone
    pub fn get_current_zone(&self) -> Result<Version, Error> {
        let url = format!("/domain/{}/version", self.name);
        let versions: Vec<Version> = execute_query(self.api_key, &url, query_set_type(HTTPOp::GET), parse_json)?;
        Ok(
            versions
            .into_iter()
            .filter(|x| x.active)
            .next()?
        )
    }

    /// Return the list of all the records in the zone 'zone'.
    pub fn get_zone_records(&self, zone: &Version) -> Result<Vec<Record>, Error> {
        let zone_url = format!("/domain/{}/version/{}/zone", self.name, zone.uuid);
        execute_query(self.api_key, &zone_url, query_set_type(HTTPOp::GET), parse_json)
    }

    /// Delete a record in 'zone' matching 'record'
    pub fn delete_record(&self, zone: &Version, record: &Record) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/zone/{}", self.name, zone.uuid, record.id);
        execute_query(self.api_key, &url, query_set_type(HTTPOp::DELETE), throw_value)?;
        Ok(())
    }
}
