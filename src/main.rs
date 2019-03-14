use clap::{App, Arg};

use dns_online::*;

fn usage() {
    eprintln!("Usage:\tle_dns_online add_record ZONE_NAME ONLINE_API_KEY RECORD_NAME TXT_VALUE");
    eprintln!("\t\tle_dns_online delete_record ZONE_NAME ONLINE_API_KEY RECORD_NAME TXT_VALUE");
}

fn main() {
    let matches = App::new("le_dns_online")
        .version("0.1")
        .author("Simon Thoby <simonthoby+ledns@gmail.com>")
        .about("Easily add or delete record in your online.net DNS zone")
        .arg(Arg::with_name("API key")
            .short("a")
            .long("api-key")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("Record")
            .short("n")
            .long("name")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("Operation")
            .short("o")
            .long("operation")
            .possible_values(&["add", "delete"])
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("Data")
            .short("d")
            .long("data")
            .takes_value(true))
        .arg(Arg::with_name("Version Name")
            .short("v")
            .long("version")
            .takes_value(true)
            .required(true))
        .get_matches();

    let api_key = matches.value_of("API key").unwrap();
    let record = {
        let mut record = matches.value_of("Record").unwrap().to_owned();
        if !record.ends_with(".") {
            record.push('.');
        }
        record
    };
    let txt_value = matches.value_of("Data");
    let zone_name = matches.value_of("Version Name").unwrap();

    let available_domains = query_available_domains(&api_key).unwrap();
    if let Some((domain, _)) = Domain::find_and_extract_path(&record, available_domains) {
        let zone = domain.get_current_zone().unwrap();
        match matches.value_of("Operation").unwrap() {
            "add" => {
                if txt_value.is_none() {
                    eprintln!("Please specify the TXT entry to add with the --data flag");
                    return;
                }
                let mut zone_entries = domain.get_zone_records(&zone).unwrap();
                zone_entries.push(Record::new(record.clone(), net::DNSType::TXT, txt_value.unwrap(), 86400));
                println!("Adding a new record to zone {} for domain {}...", zone.name, domain.name);
                let new_zone = domain.add_version(&zone_name).unwrap();
                domain.set_zone_entries(&new_zone, &zone_entries).unwrap();
                domain.enable_version(&new_zone).unwrap();
                println!("The entry {} has been deployed.", record);
            },
            "delete" => {
                println!("Suppressing the entries {} in domain {}...", record, domain.name);
                let old_zone_entries: Vec<Record> = domain.get_zone_records(&zone).unwrap();
                let new_zone_entries: Vec<Record> = old_zone_entries
                    .clone()
                    .into_iter()
                    .filter(|x| {
                        match txt_value {
                            Some(txt) => !(x.record_type == net::DNSType::TXT && x.name == record && &x.data[1..x.data.len()-1] == txt),
                            None => !(x.record_type == net::DNSType::TXT && x.name == record)
                        }
                    })
                    .collect();
                // nothing changed, so we don't do nothing
                if new_zone_entries.len() == old_zone_entries.len() {
                    println!("No such entries found, exiting...");
                    return;
                }
                let new_zone = domain.add_version(&zone_name).unwrap();
                domain.set_zone_entries(&new_zone, new_zone_entries.as_slice()).unwrap();
                domain.enable_version(&new_zone).unwrap();
                println!("The entry {} has been destroyed.", record);
            },
            _ => {
                eprintln!("Invalid action");
                usage();
            }
        }
    } else {
        eprintln!("No domain name matching {} found ! Exiting...", record);
    }
}