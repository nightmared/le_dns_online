use std::env;
use std::time::SystemTime;

use dns_online::*;

fn usage() {
    eprintln!("Usage:\tle_dns_online add_record ONLINE_API_KEY RECORD_NAME TXT_VALUE");
    eprintln!("\t\tle_dns_online delete_record ONLINE_API_KEY RECORD_NAME TXT_VALUE");
}

fn main() {
    let mut args = env::args().skip(1);
    if args.len() != 4 {
        eprintln!("Called with an invalid number of arguments (4 expected, received {})", args.len());
        usage();
        return;
    }
    let action = args.next().unwrap();
    let api_key = args.next().unwrap();
    let mut record = args.next().unwrap();
    if !record.ends_with(".") {
        record.push('.');
    }
    let txt_value = args.next().unwrap();

    let available_domains = query_available_domains(&api_key).unwrap();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    if let Some((domain, _)) = Domain::find_and_extract_path(&record, available_domains) {
        let zone = domain.get_current_zone().unwrap();
        match action.as_str() {
            "add_record" => {
                let mut zone_entries = domain.get_zone_records(&zone).unwrap();
                zone_entries.push(Record::new(record.clone(), net::DNSType::TXT, txt_value, 86400));
                let new_zone_name = format!("LE-challenge-{}", current_time);
                println!("Adding a new record to zone {} for domain {}...", zone.name, domain.name);
                let new_zone = domain.add_version(&new_zone_name).unwrap();
                domain.set_zone_entries(&new_zone, &zone_entries).unwrap();
                domain.enable_version(&new_zone).unwrap();
                println!("The entry {} has been deployed.", record);
            },
            "delete_record" => {
                println!("Suppressing the entry {} in domain {}...", record, domain.name);
                let old_zone_entries: Vec<Record> = domain.get_zone_records(&zone).unwrap();
                let new_zone_entries: Vec<Record> = old_zone_entries
                    .clone()
                    .into_iter()
                    .filter(|x| !(x.record_type == net::DNSType::TXT && x.name == record && x.data[1..x.data.len()-1] == txt_value))
                    .collect();
                // nothing changed, so we don't do nothing
                if new_zone_entries.len() == old_zone_entries.len() {
                    println!("No such entries found, exiting...");
                    return;
                }
                let new_zone_name = format!("LE-challenge-cleanup-{}", current_time);
                let new_zone = domain.add_version(&new_zone_name).unwrap();
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
