use crate::requests::entities::HttpRequestData;
use anyhow::Result;

pub trait SyncHttpRequest {
    fn call(&self, req: HttpRequestData) -> Result<String>;
}
