use curl::easy::{Easy2, Handler, List, WriteError};
use crate::error::Error;

pub struct Collector(String);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.push_str(std::str::from_utf8(data).unwrap());
        Ok(data.len())
    }
}

/// Generate a query using curl easyHTTP interface
/// This will request the api endpoint at the url api_endpoint with the user-supplied authentification token auth_token
pub fn make_query(api_endpoint: &str, auth_token: &str) -> Result<Easy2<Collector>, curl::Error> {
    let mut easy = Easy2::new(Collector(String::with_capacity(4096)));

    let mut url: String = crate::API_URL.into();
    url.push_str(api_endpoint);
    easy.url(&url)?;

    let mut http_headers = List::new();
    let mut auth: String = "Authorization: Bearer ".into();
    auth.push_str(&auth_token);
    http_headers.append(&auth)?;
    easy.http_headers(http_headers)?;
    Ok(easy)
}

/// Generate and execute an HTTP query to 'api_endpoint'.
/// This function allow you to provide a callback to configure the query (e.g. setting the type of query
/// or adding data) and another function to parse the response from the api endpoint
pub fn execute_query<T, F, F2, I: Into<Error>, I2: Into<Error>>(auth_token: &str, api_endpoint: &str, configure: F, parse: F2) -> Result<T, Error>
where F: Fn(Easy2<Collector>) -> Result<Easy2<Collector>, I> + Sized, F2: Fn(&str) -> Result<T, I2> + Sized {
    let req = make_query(api_endpoint, auth_token)?;

    let mut req = match configure(req) {
        Ok(x) => x,
        Err(e) => return Err(e.into())
    };

    req.perform()?;
    let res_code = req.response_code()?;
    if res_code < 200 || res_code >= 400 {
        return Err(Error::ApiError(req.effective_url()?.unwrap_or("").into(), req.response_code()?, req.get_ref().0.clone()));
    }

    match parse(&req.get_ref().0) {
        Ok(x) => Ok(x),
        Err(e) => Err(e.into())
    }
}

pub fn parse_json<T>(data: &str) -> Result<T, serde_json::Error> where for <'de> T: serde::Deserialize<'de> {
    Ok(serde_json::from_str(data)?)
}

pub fn throw_value(_data: &str) -> Result<(), Error> {
    Ok(())
}

/// Set the request to be made with HTTP GET
/// This can be used as a simple configuration callback function for execute_query
pub fn get_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.get(true)?;
    Ok(req)
}

/// Set the request to be made with HTTP PATCH
/// This can be used as a simple configuration callback function for execute_query
pub fn patch_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.custom_request("PATCH")?;
    Ok(req)
}

/// Set the request to be made with HTTP DELETE
/// This can be used as a simple configuration callback function for execute_query
pub fn delete_data(mut req: Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> {
    req.custom_request("DELETE")?;
    Ok(req)
}

/// This struct hold a (key, value) pair of data to be added to a new HTTP POST request
pub struct PostData<'a>(pub &'a str, pub &'a str);

/// Add a list of post (key, value) pairs to the query
pub fn post_data<'a>(data: &'a[PostData<'a>]) -> impl Fn(Easy2<Collector>) -> Result<Easy2<Collector>, curl::Error> + Sized + 'a {
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
