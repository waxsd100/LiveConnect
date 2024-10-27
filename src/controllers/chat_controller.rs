// src/controllers/chat_controller.rs

use crate::services::{chat_service::ChatService, minecraft_service::MinecraftService};
use std::sync::Arc;
use tokio_stream::StreamExt;
use crate::models::chat_model::ChatMessage;
use chrono::Local;
use anyhow::Result;

pub struct ChatController {
    chat_service: Arc<ChatService>,
    minecraft_service: Arc<MinecraftService>,
}

impl ChatController {
    pub fn new(chat_service: Arc<ChatService>, minecraft_service: Arc<MinecraftService>) -> Self {
        Self { chat_service, minecraft_service }
    }

    pub async fn run(&self) -> Result<()> {
        let stream = self.chat_service.stream_messages();

        tokio::pin!(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    // ログメッセージの生成
                    let log_message = format_log_message(&message);
                    // ログの出力
                    println!("{}", log_message);

                    // Minecraftサーバーへメッセージを送信
                    if let Err(e) = self.minecraft_service.send_chat_message(&log_message).await {
                        eprintln!("Error sending to Minecraft: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }

        Ok(())
    }
}

// ログメッセージを生成する関数
fn format_log_message(message: &ChatMessage) -> String {
    // 日時をローカルタイムゾーンでフォーマット
    let datetime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 権限記号の生成
    let mut badges = String::new();
    if message.author.is_verified {
        badges.push('✔'); // チェックマーク
    }
    if message.author.is_chat_owner {
        badges.push('👑'); // 王冠
    }
    if message.author.is_chat_sponsor {
        badges.push('💎'); // ダイヤモンド
    }
    if message.author.is_chat_moderator {
        badges.push('🔧'); // レンチ
    }

    // ログメッセージの組み立て
    format!(
        "[{datetime}] [{message_type}] {author_name}{badges}: {message}",
        datetime = datetime,
        message_type = message.message_type,
        author_name = message.author.name,
        badges = badges,
        message = message.message
    )
}
