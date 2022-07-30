use crate::requests::entities::HttpRequestData;
use anyhow::Result;
use ureq::serde::de::DeserializeOwned;

pub trait SyncHttpRequest {
    fn call(&self, req: HttpRequestData) -> Result<String>;
}
