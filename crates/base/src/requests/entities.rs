use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum HttpRequestMethod {
    Get,
    Post,
}

impl Default for HttpRequestMethod {
    fn default() -> Self {
        Self::Get
    }
}

pub type Header = String;
pub type HeaderValue = String;

pub type Query = String;
pub type QueryValue = String;

pub type Headers = HashMap<Header, HeaderValue>;
pub type Queries = HashMap<Query, QueryValue>;

pub type Url = String;

#[derive(Debug, Clone, Default)]
pub struct HttpRequestData {
    pub method: HttpRequestMethod,
    pub url: Url,
    pub headers: Option<Headers>,
    pub queries: Option<Queries>,
    pub body: Option<Value>,
}

impl HttpRequestData {
    pub fn new(method: HttpRequestMethod, url: impl Into<Url>) -> Self {
        HttpRequestData {
            method,
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn add_header(mut self, header: impl Into<Header>, value: impl Into<HeaderValue>) -> Self {
        if self.headers.is_none() {
            self.headers = Some(HashMap::new());
        }

        self.headers
            .as_mut()
            .unwrap()
            .insert(header.into(), value.into());

        self
    }

    pub fn add_query(mut self, query: impl Into<Query>, value: impl Into<QueryValue>) -> Self {
        if self.queries.is_none() {
            self.queries = Some(HashMap::new());
        }

        self.queries
            .as_mut()
            .unwrap()
            .insert(query.into(), value.into());

        self
    }

    pub fn with_json_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
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
