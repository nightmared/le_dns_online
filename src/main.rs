extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use curl::easy::{Easy2, Handler, List, WriteError};
use std::{env, fmt, convert};
use std::time::SystemTime;
use serde::{Deserialize, Deserializer, de::Visitor};

static API_URL: &'static str = "https://api.online.net/api/v1";

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
    eprintln!("\t{} delete_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
}

#[derive(Deserialize, Debug)]
struct Domain<'a> {
    #[serde(skip)]
    api_key: &'a str,
    id: usize,
    name: String,
    dnssec: bool,
    external: bool
}

// We need to implement this special type because the API can return the tty both as a number and
// as a string !?
#[derive(Debug)]
struct TTL {
    val: usize
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
struct Record {
    //id: usize,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    ttl: TTL,
    data: String
}

#[derive(Deserialize, Debug)]
struct Version {
    uuid_ref: String,
    name: String,
    active: bool
}

struct Collector(String);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.push_str(std::str::from_utf8(data).unwrap());
        Ok(data.len())
    }
}


fn make_query(api_endpoint: &str, auth_token: &str) -> Result<Easy2<Collector>, curl::Error> {
    let mut easy = Easy2::new(Collector(String::with_capacity(4096)));

    let mut url: String = API_URL.into();
    url.push_str(api_endpoint);
    easy.url(&url)?;

    let mut http_headers = List::new();
    let mut auth: String = "Authorization: Bearer ".into();
    auth.push_str(&auth_token);
    http_headers.append(&auth)?;
    easy.http_headers(http_headers)?;
    Ok(easy)
}

enum Error {
    CurlError(curl::Error),
    SerdeError(serde_json::Error),
    // ApiError(url, status_code, body)
    ApiError((String, u32, String))
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::CurlError(ref e) => write!(f, "CurlError({:?})", e),
            &Error::SerdeError(ref e) => write!(f, "SerdeError({:?}", e),
            &Error::ApiError((ref url, status_code, ref body)) => write!(f, "ApiError(url={:?}&status_code={}&body={})", url, status_code, body)
        }
    }
}

impl convert::From<curl::Error> for Error {
    fn from(e: curl::Error) -> Error {
        Error::CurlError(e)
    }
}

impl convert::From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::SerdeError(e)
    }
}


// Generate and execute an HTTP query to 'api_endpoint'.
// This function allow you tu provide a function to configure the query (e.g. setting the type of query
// or adding data) and another function to parse the api response
fn execute_query<T, F, F2, I: Into<Error>, I2: Into<Error>>(auth_token: &str, api_endpoint: &str, configure: F, parse: F2) -> Result<T, Error>
where F: Fn(Easy2<Collector>) -> Result<Easy2<Collector>, I> + Sized, F2: Fn(&str) -> Result<T, I2> + Sized {
    let req = make_query(api_endpoint, auth_token)?;

    let mut req = match configure(req) {
        Ok(x) => x,
        Err(e) => return Err(e.into())
    };

    req.perform()?;
    let res_code = req.response_code()?;
    if res_code < 200 || res_code >= 400 {
        return Err(Error::ApiError((req.effective_url()?.unwrap_or("").into(), req.response_code()?, req.get_ref().0.clone())));
    }

    match parse(&req.get_ref().0) {
        Ok(x) => Ok(x),
        Err(e) => Err(e.into())
    }
}

fn parse_json<T>(data: &str) -> Result<T, serde_json::Error> where for <'de> T: serde::Deserialize<'de> {
    Ok(serde_json::from_str(data)?)
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

fn get_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.get(true)?;
    Ok(req)
}

fn patch_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.custom_request("PATCH")?;
    Ok(req)
}

fn delete_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.custom_request("DELETE")?;
    Ok(req)
}

// PostData(name, value)
struct PostData<'a>(&'a str, &'a str);

fn post_data<'a>(data: &'a[PostData<'a>]) -> impl Fn(Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> + Sized +'a {
    move |mut req: Easy2<Collector>| {
        req.post(true)?;
        let mut post_fields = String::with_capacity(data.len()*25);
        for e in data {
            let entry = format!("{}={}&", req.url_encode(e.0.as_bytes()), req.url_encode(e.1.as_bytes()));
            post_fields.push_str(&entry);
        }
        // delete the last '&' if any
        post_fields.pop();

        req.post_field_size(post_fields.len() as u64)?;
        req.post_fields_copy(post_fields.as_bytes())?;
        Ok(req)
    }
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
        let zone_ref = self.get_current_zone()?.into_iter().filter(|x| x.name == matching_record).next().unwrap();
        let versions = self.get_versions()?;
        let tmp_version = versions.iter().filter(|x| x.active).next().unwrap();
        let original_version = versions.iter().filter(|x| x.uuid_ref == zone_ref.data).next().unwrap();
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
