use std::env;
use std::time::SystemTime;

use lib::*;

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
    eprintln!("\t\t{} delete_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
}

fn main() {
    let mut args = env::args();
    let app_name = args.next().unwrap();
    if args.len() != 3 {
        eprintln!("Called with an invalid number of arguments (3 expected, received {})", args.len());
        usage(&app_name);
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
                domain.add_record(&zone, record.clone(), "TXT", txt_value, 86400).unwrap();
            },
            "delete_record" => {
                while let Some(records) = domain.get_record(&zone, &record, Some(&txt_value)).unwrap() {
                    domain.delete_record(&zone, &records[0]).unwrap();
                }
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
