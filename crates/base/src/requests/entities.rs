use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum HttpRequestType {
    Get,
}

impl Default for HttpRequestType {
    fn default() -> Self {
        Self::Get
    }
}

pub type Headers<'a> = HashMap<&'a str, &'a str>;
pub type Queries<'a> = HashMap<&'a str, &'a str>;

#[derive(Debug, Clone, Default)]
pub struct HttpRequestData<'a> {
    pub req_type: HttpRequestType,
    pub url: &'a str,
    pub headers: Headers<'a>,
    pub queries: Queries<'a>,
}

pub type NumberOfRetries = u32;
pub type SecondsToSleep = u32;

#[derive(Default)]
pub struct HttpRequestWithRetriesParams<'a> {
    pub req_entity_name: &'a str,
    pub target_logger: &'a str,
    pub number_of_retries: NumberOfRetries,
    pub seconds_to_sleep: SecondsToSleep,
}
