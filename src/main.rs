// src/main.rs

mod controllers;
mod models;
mod repositories;
mod services;

use controllers::chat_controller::ChatController;
use services::chat_service::ChatService;
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 監視したいYouTube動画のID
    let video_id = "VIDEO-ID";

    // チャットサービスの初期化
    let chat_service = ChatService::new(video_id).await?;
    let chat_service = Arc::new(chat_service);

    // コントローラの作成
    let controller = ChatController::new(chat_service);

    // チャットの取得と表示を開始
    controller.run().await?;

    Ok(())
}
