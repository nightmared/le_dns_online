use std::env;

use lib::*;

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
    eprintln!("\t{} delete_record ONLINE_API_KEY RECORD_NAME TXT_VALUE", app_name);
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
    if let Some((domain, subrecord)) = Domain::find_and_extract_path(&record, available_domains) {
        match action.as_str() {
            "add_record" => {
                let zone = domain.get_current_zone().unwrap();
                domain.add_record(&zone, subrecord, "TXT", txt_value, 86400).unwrap();
            },
            "delete_record" => {
                let zone = domain.get_current_zone().unwrap();
                if let Some(records) = domain.get_record(&zone, &subrecord, Some(&txt_value)).unwrap() {
                    for ref e in records {
                        domain.delete_record(&zone, e).unwrap();
                    }
                }

                //domain.delete_record(&subrecord, &txt_value).unwrap();
            },
            _ => {
                eprintln!("Invalid action");
                usage(&app_name);
            }
        }
    } else {
        eprintln!("Aucun nom de domaine correspondant à {} n'a pu être trouvé", record);
    }
}
