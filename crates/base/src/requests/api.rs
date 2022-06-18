use crate::requests::entities::HttpRequestData;
use anyhow::Result;
use ureq::serde::de::DeserializeOwned;

pub trait HttpRequest {
    fn call<T>(&self, req: HttpRequestData) -> Result<T>
    where
        T: DeserializeOwned;
}
