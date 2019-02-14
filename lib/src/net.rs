use curl::easy::{Easy, List};
use crate::error::{Error, APIError};

/// This struct hold a (key, value) tuple of data to send along a HTTP POST or PATCH request
pub struct PostData<'a>(pub &'a str, pub &'a str);

/// This enum contains all the kinds of operations supported by the API
/// You have to specify data if you are using an operation that requires it, such as POST.
pub enum HTTPOp<'a> {
    GET,
    POST(&'a [PostData<'a>]),
    PATCH(Option<&'a [PostData<'a>]>),
    DELETE
}

/// Generate a query using curl easyHTTP interface
/// This will request the api endpoint at the url api_endpoint with the user-supplied authentification token auth_token
pub fn make_query(api_endpoint: &str, auth_token: &str) -> Result<Easy, curl::Error> {
    let mut easy = Easy::new();

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

fn attach_data(req: &mut Easy, data: &[PostData]) -> Result<(), Error> {
    if data.len() == 0 {
        return Err(Error::InvalidPost);
    }
    // the data.len()*25 is just a very rough heuristic
    let mut post_fields = String::with_capacity(data.len()*25);
    for e in data {
        let entry = format!("{}={}&", req.url_encode(e.0.as_bytes()), req.url_encode(e.1.as_bytes()));
        post_fields.push_str(&entry);
    }
    // delete the last '&'
    post_fields.pop();

    req.post_field_size(post_fields.len() as u64)?;
    req.post_fields_copy(post_fields.as_bytes())?;
    Ok(())
}

/// Select the type of HTTP operation to perform.
/// This can be used as a simple configuration callback function for execute_query.
pub fn query_set_type<'a>(http_operation: HTTPOp<'a>) -> impl Fn(Easy) -> Result<Easy, Error> + 'a {
    move |mut req: Easy| {
        match http_operation {
            HTTPOp::GET => req.get(true)?,
            HTTPOp::DELETE => req.custom_request("DELETE")?,
            HTTPOp::PATCH(data) => {
                req.custom_request("PATCH")?;
                if let Some(data) = data {
                    attach_data(&mut req, data)?;
                }
            },
            HTTPOp::POST(data) => {
                req.post(true)?;
                attach_data(&mut req, data);
            }
        }
        Ok(req)
    }
}

/// Generate and execute an HTTP query to 'api_endpoint'.
/// This function allow you to provide a callback to configure the query (e.g. setting the type of query
/// or adding data) and another function to parse the response from the api endpoint
pub fn execute_query<T, F, F2, I: Into<Error>, I2: Into<Error>>(auth_token: &str, api_endpoint: &str, configure: F, parse: F2) -> Result<T, Error>
where F: Fn(Easy) -> Result<Easy, I> + Sized, F2: Fn(&[u8]) -> Result<T, I2> + Sized {
    let req = make_query(api_endpoint, auth_token)?;

    let mut req = match configure(req) {
        Ok(x) => x,
        Err(e) => return Err(e.into())
    };

    let mut buf = Vec::new();
    {
        let mut transfer = req.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();

        transfer.perform()?;
    }
    let res_code = req.response_code()?;
    if res_code < 200 || res_code >= 400 {
        return Err(Error::ApiError(APIError {
			url: req.effective_url()?.unwrap_or("<UNKNOWN URL>").into(),
            status_code: res_code,
            body: buf
        }));
    }

    match parse(&buf) {
        Ok(x) => Ok(x),
        Err(e) => Err(e.into())
    }
}

/// Return the json object parsed as a Rust object of type T
pub fn parse_json<T>(data: &[u8]) -> Result<T, serde_json::Error> where for <'de> T: serde::Deserialize<'de> {
    Ok(serde_json::from_slice(data)?)
}

/// We don't care about this value, so we might as well throw it away. Note that you may still
/// have to annotate the type T for your module to compile.
pub fn throw_value(_data: &[u8]) -> Result<(), Error> {
    Ok(())
}
