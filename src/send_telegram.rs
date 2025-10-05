use crate::{ models::TelegramConfig};
use rustygram::types::{SendMessageOption, SendMessageParseMode};
use log::debug;


pub async fn send_telegram_msg(
    message_text: &String,
    telegram_config: &TelegramConfig,
) {
    debug!("Sending message to telegram {} {} {}",message_text ,telegram_config.bot_token , telegram_config.channel_id ) ;

    
 // telegram configuration 
    let bot_token = telegram_config.bot_token.clone();
    let channel_id= telegram_config.channel_id.clone();
    let instance = rustygram::create_bot(&bot_token, &channel_id);
     // send a simple text message
    let option = SendMessageOption { parse_mode: Some(SendMessageParseMode::MarkdownV2) };

    rustygram::send_message(&instance, message_text, Some(option)).await.unwrap();


}
