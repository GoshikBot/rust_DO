use base::notifier::{Notifier, TelegramNotifier};
use base::requests::ureq::UreqRequestApi;

#[test]
#[allow(non_snake_case)]
fn send_message__ureq_telegram_notifier__successful_response() {
    dotenv::from_filename("common.env").unwrap();

    let telegram_bot_token = dotenv::var("TELEGRAM_BOT_TOKEN").unwrap();
    let telegram_bot_chat_id = dotenv::var("TELEGRAM_BOT_CHAT_ID").unwrap();

    let notifier = TelegramNotifier::new(
        telegram_bot_token,
        telegram_bot_chat_id,
        UreqRequestApi::new(),
    );
    notifier.send_message("test").unwrap();
}
