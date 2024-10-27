// src/services/chat_service.rs

use crate::repositories::chat_repository::ChatRepository;
use crate::models::chat_model::ChatMessage;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use futures::stream::BoxStream;
use std::time::Duration;

pub struct ChatService {
    repository: Arc<ChatRepository>,
    continuation: Arc<Mutex<Option<String>>>,
    api_key: String,
    client_version: String,
}

impl ChatService {
    pub async fn new(video_id: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()?;
        let repository = Arc::new(ChatRepository::new(client));

        // 初期データの取得
        let (continuation, api_key, client_version) = repository.get_initial_data(video_id).await?;

        Ok(Self {
            repository,
            continuation: Arc::new(Mutex::new(Some(continuation))),
            api_key,
            client_version,
        })
    }

    pub fn stream_messages(&self) -> BoxStream<'static, Result<ChatMessage>> {
        let repository = self.repository.clone();
        let continuation_mutex = self.continuation.clone();
        let api_key = self.api_key.clone();
        let client_version = self.client_version.clone();

        Box::pin(async_stream::stream! {
            loop {
                let continuation = {
                    let lock = continuation_mutex.lock().await;
                    lock.clone()
                };

                if let Some(continuation_token) = continuation {
                    let (messages, next_continuation, timeout) = match repository.get_chat_messages(
                        &continuation_token,
                        &api_key,
                        &client_version,
                    ).await {
                        Ok(data) => data,
                        Err(e) => {
                            eprintln!("Error fetching chat messages: {}", e);
                            break;
                        }
                    };

                    for message in messages {
                        yield Ok(message);
                    }

                    // continuationトークンの更新
                    {
                        let mut lock = continuation_mutex.lock().await;
                        *lock = Some(next_continuation);
                    }

                    // 待機時間の調整（最大でも2秒）
                    let sleep_duration = Duration::from_millis(timeout.min(2000));
                    tokio::time::sleep(sleep_duration).await;
                } else {
                    eprintln!("Continuation token is missing. Exiting the loop.");
                    break;
                }
            }
        })
    }
}
