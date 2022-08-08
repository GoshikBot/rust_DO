use crate::requests::api::SyncHttpRequest;
use crate::requests::entities::{HttpRequestData, HttpRequestMethod};
use anyhow::{bail, Result};
use ureq::Error;

#[derive(Default)]
pub struct UreqRequestApi {}

impl UreqRequestApi {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SyncHttpRequest for UreqRequestApi {
    fn call(&self, req: HttpRequestData) -> Result<String> {
        let req_fn = match req.method {
            HttpRequestMethod::Get => ureq::get,
            HttpRequestMethod::Post => ureq::post,
        };

        let mut request = req_fn(&req.url);
        if let Some(headers) = &req.headers {
            for (header, value) in headers {
                request = request.set(header, value);
            }
        }

        if let Some(queries) = &req.queries {
            for (param, value) in queries {
                request = request.query(param, value);
            }
        }

        let res = if let Some(body) = req.body {
            request.send_json(body)
        } else {
            request.call()
        };

        match res {
            Ok(resp) => Ok(resp.into_string()?),
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
