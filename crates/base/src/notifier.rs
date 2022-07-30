use crate::requests::api::SyncHttpRequest;
use crate::requests::entities::{HttpRequestData, HttpRequestMethod};
use anyhow::Result;
use serde_json::json;

pub trait Notifier {
    fn send_message(&self, message: &str) -> Result<()>;
}

pub struct TelegramNotifier<R: SyncHttpRequest> {
    token: String,
    chat_id: String,
    request_api: R,
}

impl<R: SyncHttpRequest> TelegramNotifier<R> {
    pub fn new(token: String, chat_id: String, request_api: R) -> TelegramNotifier<R> {
        TelegramNotifier {
            token,
            chat_id,
            request_api,
        }
    }
}

impl<R: SyncHttpRequest> Notifier for TelegramNotifier<R> {
    fn send_message(&self, message: &str) -> Result<()> {
        let req = HttpRequestData::new(
            HttpRequestMethod::Post,
            &format!(
                "https://api.telegram.org/bot{token}/sendMessage",
                token = &self.token
            ),
        )
        .with_json_body(json!({
            "text": message,
            "chat_id": self.chat_id
        }));

        self.request_api.call(req)?;
        Ok(())
    }
}
