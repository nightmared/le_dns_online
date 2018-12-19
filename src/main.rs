use std::env;

mod api;
mod net;
mod config;
#[cfg(test)]
mod test;
use crate::api::*;

fn usage(app_name: &str) {
    eprintln!("Usage:\t{} add_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
    eprintln!("\t{} delete_record ONLINE_API_KEY RECORD TXT_VALUE", app_name);
}

/// Try to extract the longest matching domain from the list of our available domains and the internal part of the name.
/// e.g. extract_domain("this.is.a.dummy.test.fr", {Domain("test.fr"), Domain("nope.fr")}) should return
/// the domain associated with "test.fr" and the internal path, aka "this.is.a.dummy."
fn extract_domain<'a>(full_domain_name: &'a str, domains: Vec<Domain<'a>>) -> Option<(Domain<'a>, &'a str)> {
    for available_domain in domains {
        if full_domain_name.ends_with(&available_domain.name) {
            let max_len = full_domain_name.len()-available_domain.name.len()-1;
            return Some((available_domain, &full_domain_name[0..max_len]));
        }
    }
    None
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
    if let Some((domain, subrecord)) = extract_domain(&record, available_domains) {
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
