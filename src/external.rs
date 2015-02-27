use std::error::{self, Error};
use std::fmt;
use std::old_io::IoError;
use std::old_io::net::ip::{IpAddr, ToSocketAddr};
use std::str;

use curl::ErrCode;
use curl::http;

// Content of the request.
const EXTERNAL_IP_REQUEST: &'static str =
"<SOAP-ENV:Envelope SOAP-ENV:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\" xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\">
    <SOAP-ENV:Body>
        <m:GetExternalIPAddress xmlns:m=\"urn:schemas-upnp-org:service:WANIPConnection:1\">
        </m:GetExternalIPAddress>
    </SOAP-ENV:Body>
</SOAP-ENV:Envelope>";

// Content of the SOAPAction header.
const SOAP_ACTION: &'static str = "\"urn:schemas-upnp-org:service:WANIPConnection:1#GetExternalIPAddress\"";

// Errors
#[derive(Debug)]
pub enum RequestError {
    ErrCode(ErrCode),
    InvalidResponse,
    IoError(IoError),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RequestError::ErrCode(ref err) => err.fmt(f),
            RequestError::InvalidResponse => write!(f, "{}", self.description()),
            RequestError::IoError(ref err) => err.fmt(f),
        }
    }
}

impl error::FromError<ErrCode> for RequestError {
    fn from_error(err: ErrCode) -> RequestError {
        RequestError::ErrCode(err)
    }
}

impl error::FromError<IoError> for RequestError {
    fn from_error(err: IoError) -> RequestError {
        RequestError::IoError(err)
    }
}

impl Error for RequestError {
    fn description(&self) -> &str {
        match *self {
            RequestError::ErrCode(ref err) => err.description(),
            RequestError::InvalidResponse => "Invalid response received from router",
            RequestError::IoError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            RequestError::ErrCode(ref err) => err.cause(),
            RequestError::InvalidResponse => None,
            RequestError::IoError(ref err) => err.cause(),
        }
    }
}

// Get the external IP address.
pub fn get_external_ip<A: ToSocketAddr>(to_addr: A) -> Result<IpAddr, RequestError>  {
    let addr = try!(to_addr.to_socket_addr());
    let url = format!("http://{}:{}/", addr.ip, addr.port);
    let resp = try!(http::handle()
        .post(url, EXTERNAL_IP_REQUEST)
        .header("SOAPAction", SOAP_ACTION)
        .exec());
    let text = str::from_utf8(resp.get_body()).unwrap(); // TODO Shouldn't, but can fail.
    extract_address(text)
}

// Extract the address from the text.
fn extract_address(text: &str) -> Result<IpAddr, RequestError> {
    let re = regex!(r"<NewExternalIPAddress>(\d+\.\d+\.\d+\.\d+)</NewExternalIPAddress>");
    match re.captures(text) {
        None => Err(RequestError::InvalidResponse),
        Some(cap) => {
            match cap.at(1) {
                None => Err(RequestError::InvalidResponse),
                Some(ip) => Ok(ip.parse::<IpAddr>().unwrap()),
            }
        },
    }
}