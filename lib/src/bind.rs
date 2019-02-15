use crate::api::Record;
use crate::net::DNSType;

/// Generate a string made a entry in a form similar to
/// "host2.homelab.local.        IN      A       10.1.100.91" 
pub fn to_bind(records: &[Record]) -> String {
    let mut res = String::new();
    for r in records {
        let record_bind = 
            if r.record_type == DNSType::MX {
                format!("{} IN MX 10 {}\n", r.name, r.data)
            } else {
                format!("{} IN {} {}\n", r.name, &String::from(&r.record_type), r.data)
            };
        res.push_str(&record_bind);
    }
    res
}
