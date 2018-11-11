extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use std::env;
use std::time::SystemTime;

mod config;
mod api;
mod net;
use crate::api::*;
use crate::net::*;

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
    eprintln!("\t{} delete_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
}


// Extract two parts from the domain to be used in LE auth check:
// the domain on which we must act and the record name
fn extract_domain<'a>(domain: &'a str, domains: Vec<Domain<'a>>) -> Option<(Domain<'a>, String)> {
    for available_domain in domains {
        if domain.ends_with(&available_domain.name) {
            let max_len = domain.len()-available_domain.name.len()-1;
            return Some((available_domain, domain[0..max_len].into()));
        }
    }
    None
}

impl<'a> Domain<'a> {
    fn add_version(&self, name: &str) -> Result<Version, Error> {
        let domain_version_url = format!("/domain/{}/version", self.name);
        let domain_version_post_data = vec![PostData("name", &name)];
        execute_query(self.api_key, &domain_version_url, post_data(&domain_version_post_data), parse_json)
    }

    // Append a record to the zone corresponding to destination
    fn append_record(&self, destination: &Version, record: &Record) -> Result<Record, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let ttl = record.ttl.as_string();
        let post_entries = vec![PostData("name", &record.name), PostData("type", &record.record_type), PostData("priority", "12"), PostData("ttl", &ttl), PostData("data", &record.data)];
        execute_query(self.api_key, &dest_zone_url, post_data(&post_entries), parse_json)
    }

    // copy all the records from source to destination and return the new zone records
    fn copy_zone(&self, source: Vec<Record>, destination: &Version) -> Result<Vec<Record>, Error> {
        let dest_zone_url = format!("/domain/{}/version/{}/zone", self.name, destination.uuid_ref);
        let mut dest_zone: Vec<Record> = execute_query(self.api_key, &dest_zone_url, get_data, parse_json)?;
        for ref entry in source {
            dest_zone.push(self.append_record(destination, entry)?);
        }
        Ok(dest_zone)
    }

    fn enable_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}/enable", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, patch_data, |_| -> Result<(), Error> { Ok(()) })
    }

    fn delete_version(&self, v: &Version) -> Result<(), Error> {
        let url = format!("/domain/{}/version/{}", self.name, v.uuid_ref);
        execute_query(self.api_key, &url, delete_data, |_| -> Result<(), Error> { Ok(()) })
    }

    fn get_versions(&self) -> Result<Vec<Version>, Error> {
        let url = format!("/domain/{}/version", self.name);
        execute_query(self.api_key, &url, get_data, parse_json)
    }

    fn get_current_zone(&self) -> Result<Vec<Record>, Error> {
        let zone_url = format!("/domain/{}/zone", self.name);
        execute_query(self.api_key, &zone_url, get_data, parse_json)
    }

    fn add_record(self, subrecord: &str, txt_value: &str) -> Result<(), Error> {
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

    fn delete_record(self, subrecord: &str, _txt_value: &str) -> Result<(), Error> {
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

fn main() {
    let mut args = env::args();
    let app_name = args.next().unwrap();
    if args.len() != 4 {
        eprintln!("Called with an invalid number of arguments");
        usage(&app_name);
        return;
    }
    let action = args.next().unwrap();
    let api_key = args.next().unwrap();
    let record = args.next().unwrap();
    let txt_value = args.next().unwrap();

    let available_domains = execute_query(&api_key, "/domain/", get_data, parse_json).unwrap();
    if let Some((mut domain, subrecord)) = extract_domain(&record, available_domains) {
        domain.api_key = &api_key;
        match action.as_str() {
            "add_record" => { domain.add_record(&subrecord, &txt_value).unwrap(); },
            "delete_record" => { domain.delete_record(&subrecord, &txt_value).unwrap(); },
            _ => {
                eprintln!("Invalid action");
                usage(&app_name);
            }
        }
    } else {
        eprintln!("Aucun nom de domaine correspondant à {} n'a pu être trouvé", record);
    }
}
