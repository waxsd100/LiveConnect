// src/models/chat_model.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub message_type: String,
    pub id: String,
    pub message: String,
    pub message_ex: Option<Vec<MessageRun>>,
    pub timestamp: i64,
    pub datetime: String,
    pub elapsed_time: Option<String>,
    pub amount_value: Option<f64>,
    pub amount_string: Option<String>,
    pub currency: Option<String>,
    pub bg_color: Option<u32>,
    pub author: Author,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageRun {
    pub text: Option<String>,
    pub emoji: Option<Emoji>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Emoji {
    pub id: String,
    pub txt: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Author {
    pub name: String,
    pub channel_id: String,
    pub channel_url: String,
    pub image_url: String,
    pub badge_url: Option<String>,
    pub is_verified: bool,
    pub is_chat_owner: bool,
    pub is_chat_sponsor: bool,
    pub is_chat_moderator: bool,
}
