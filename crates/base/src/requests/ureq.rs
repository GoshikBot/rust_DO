use crate::requests::api::HttpRequest;
use crate::requests::entities::{HttpRequestData, HttpRequestType, Queries};
use anyhow::{bail, Result};
use ureq::serde::de::DeserializeOwned;
use ureq::Error;

#[derive(Default)]
pub struct UreqRequestApi {}

impl UreqRequestApi {
    pub fn new() -> Self {
        Default::default()
    }
}

impl HttpRequest for UreqRequestApi {
    fn call<T>(&self, req: HttpRequestData) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let req_fn = match req.req_type {
            HttpRequestType::Get => ureq::get,
        };

        let mut request = req_fn(req.url);
        for (header, value) in &req.headers {
            request = request.set(header, value);
        }

        for (param, value) in &req.queries {
            request = request.query(param, value);
        }

        let res = request.call();

        match res {
            Ok(resp) => Ok(resp.into_json()?),
            Err(e) => match e {
                Error::Status(code, resp) => {
                    bail!(
                        "request to {} failed with a code {}: {}",
                        resp.get_url().to_string(),
                        code,
                        resp.into_string()?
                    );
                }
                e => bail!(e),
            },
        }
    }
}
