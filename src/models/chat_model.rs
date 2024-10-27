// src/models/chat_model.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    // 共通フィールド
    pub message_type: String,   // メッセージの種類
    pub id: String,             // メッセージID
    pub message: String,        // メッセージ内容
    pub message_ex: Option<Vec<MessageRun>>, // 絵文字情報を含む詳細なメッセージ
    pub timestamp: i64,         // Unixタイムスタンプ（ミリ秒）
    pub datetime: String,       // 日時（文字列）
    pub elapsed_time: Option<String>, // 経過時間（リプレイのみ）
    pub amount_value: Option<f64>,    // 金額（数値）
    pub amount_string: Option<String>,// 金額（文字列）
    pub currency: Option<String>,     // 通貨コード
    pub bg_color: Option<u32>,        // 背景色（RGB Int）

    // 投稿者情報
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
