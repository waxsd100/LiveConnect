// src/main.rs

mod controllers;
mod models;
mod repositories;
mod services;

use controllers::chat_controller::ChatController;
use services::{chat_service::ChatService, minecraft_service::MinecraftService};
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 監視したいYouTube動画のID
    let video_id = "YOUR_VIDEO_ID";

    // Minecraftサーバーのアドレス
    let minecraft_server_addr = "YOUR_MINECRAFT_SERVER_ADDRESS";

    // チャットサービスの初期化
    let chat_service = ChatService::new(video_id).await?;
    let chat_service = Arc::new(chat_service);

    // Minecraftサービスの初期化
    let minecraft_service = MinecraftService::new(minecraft_server_addr)?;
    let minecraft_service = Arc::new(minecraft_service);

    // コントローラの作成
    let controller = ChatController::new(chat_service, minecraft_service);

    // チャットの取得と送信を開始
    controller.run().await?;

    Ok(())
}
