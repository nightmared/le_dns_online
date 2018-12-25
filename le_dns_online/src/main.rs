use std::env;
use std::time::SystemTime;

use lib::*;

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
    eprintln!("\t{} delete_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
}

fn create_and_copy_current_zone(domain: &Domain, version_name: String) -> Version {
    let zone = domain.get_current_zone().unwrap();
    let new_zone = domain.add_version(&version_name).unwrap();
    domain.copy_zone(domain.get_zone_records(&zone).unwrap(), &new_zone).unwrap();
    new_zone
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

    let available_domains = query_available_domains(&api_key).unwrap();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    if let Some((domain, _)) = Domain::find_and_extract_path(&record, available_domains) {
        match action.as_str() {
            "add_record" => {
                let new_zone = create_and_copy_current_zone(&domain, format!("LE-challenge-{}", current_time));
                domain.add_record(&new_zone, record.clone(), "TXT", txt_value, 86400).unwrap();
                domain.enable_version(&new_zone).unwrap();
            },
            "delete_record" => {
                let new_zone = create_and_copy_current_zone(&domain, format!("LE-challenge-{}-validated", current_time));
                if let Some(records) = domain.get_record(&new_zone, &record, Some(&txt_value)).unwrap() {
                    for ref e in records {
                        domain.delete_record(&new_zone, e).unwrap();
                    }
                }
                domain.enable_version(&new_zone).unwrap();
            },
            _ => {
                eprintln!("Invalid action");
                usage(&app_name);
            }
        }
    } else {
        eprintln!("No domain name matching {} found ! Exiting...", record);
    }
}
