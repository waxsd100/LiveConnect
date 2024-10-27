// src/repositories/chat_repository.rs

use crate::models::chat_model::*;
use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;

pub struct ChatRepository {
    client: Client,
}

impl ChatRepository {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn get_initial_data(&self, video_id: &str) -> Result<(String, String, String)> {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?
            .text()
            .await?;

        // ytInitialDataを抽出する正規表現
        let re = Regex::new(r#"var ytInitialData = (\{.*?});"#)?;
        let caps = re
            .captures(&resp)
            .ok_or_else(|| anyhow!("Failed to extract ytInitialData"))?;
        let yt_initial_data = &caps[1];

        // JSONをパース
        let v: Value = serde_json::from_str(yt_initial_data)?;

        // continuationトークンの取得
        let continuation = v["contents"]["twoColumnWatchNextResults"]["conversationBar"]
            ["liveChatRenderer"]["continuations"][0]["reloadContinuationData"]["continuation"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get continuation token"))?
            .to_string();

        // APIキーの取得
        let re_api_key = Regex::new(r#"["']INNERTUBE_API_KEY["']\s*:\s*["']([^"']+)["']"#)?;
        let api_key = re_api_key
            .captures(&resp)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow!("Failed to extract API key"))?;

        // clientVersionの取得
        let re_client_version =
            Regex::new(r#"["']INNERTUBE_CONTEXT_CLIENT_VERSION["']\s*:\s*["']([^"']+)["']"#)?;
        let client_version = re_client_version
            .captures(&resp)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow!("Failed to extract client version"))?;

        Ok((continuation, api_key, client_version))
    }

    pub async fn get_chat_messages(
        &self,
        continuation: &str,
        api_key: &str,
        client_version: &str,
    ) -> Result<(Vec<ChatMessage>, String, u64)> {
        let url = format!(
            "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
            api_key
        );

        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": client_version
                }
            },
            "continuation": continuation
        });

        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "Mozilla/5.0")
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let mut messages = Vec::new();
        if let Some(actions) =
            resp["continuationContents"]["liveChatContinuation"]["actions"].as_array()
        {
            for action in actions {
                if let Some(add_chat_item_action) = action.get("addChatItemAction") {
                    let item = &add_chat_item_action["item"];

                    // すべてのメッセージタイプを処理
                    if let Some(renderer) = item.get("liveChatTextMessageRenderer") {
                        if let Some(chat_message) = self.parse_text_message(renderer) {
                            messages.push(chat_message);
                        }
                    } else if let Some(renderer) = item.get("liveChatPaidMessageRenderer") {
                        if let Some(chat_message) = self.parse_paid_message(renderer, "superChat") {
                            messages.push(chat_message);
                        }
                    } else if let Some(renderer) = item.get("liveChatPaidStickerRenderer") {
                        if let Some(chat_message) =
                            self.parse_paid_message(renderer, "superSticker")
                        {
                            messages.push(chat_message);
                        }
                    } else if let Some(renderer) = item.get("liveChatMembershipItemRenderer") {
                        if let Some(chat_message) = self.parse_membership_message(renderer) {
                            messages.push(chat_message);
                        }
                    } else if let Some(renderer) =
                        item.get("liveChatViewerEngagementMessageRenderer")
                    {
                        if let Some(chat_message) = self.parse_viewer_engagement_message(renderer) {
                            messages.push(chat_message);
                        }
                    } else if let Some(_renderer) = item.get("liveChatPlaceholderItemRenderer") {
                        // プレースホルダーメッセージは無視
                    } else {
                        // 未対応のメッセージタイプをログに出力
                        eprintln!("Unsupported message type: {:?}", item);
                    }
                }
            }
        }

        // Continuationトークンと待機時間の取得
        let mut next_continuation = String::new();
        let mut timeout = 0;

        if let Some(continuations) =
            resp["continuationContents"]["liveChatContinuation"]["continuations"].as_array()
        {
            for continuation_data in continuations {
                if let Some(timed_data) = continuation_data.get("timedContinuationData") {
                    next_continuation = timed_data["continuation"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    timeout = timed_data["timeoutMs"].as_u64().unwrap_or(0);
                    break;
                } else if let Some(invalidation_data) =
                    continuation_data.get("invalidationContinuationData")
                {
                    next_continuation = invalidation_data["continuation"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    timeout = invalidation_data["timeoutMs"].as_u64().unwrap_or(0);
                    break;
                } else if let Some(reload_data) = continuation_data.get("reloadContinuationData") {
                    next_continuation = reload_data["continuation"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    timeout = reload_data["timeoutMs"].as_u64().unwrap_or(0);
                    break;
                }
            }
        }

        if next_continuation.is_empty() {
            return Err(anyhow!("Failed to retrieve next continuation token"));
        }

        Ok((messages, next_continuation, timeout))
    }

    // テキストメッセージの解析
    fn parse_text_message(&self, renderer: &Value) -> Option<ChatMessage> {
        let author = self.parse_author(
            &renderer["authorName"],
            &renderer["authorPhoto"],
            &renderer["authorBadges"],
        )?;
        let message_runs = self.parse_message_runs(&renderer["message"]["runs"]);

        let message = message_runs
            .iter()
            .filter_map(|run| run.text.clone())
            .collect::<String>();

        let message_ex = Some(message_runs);

        Some(ChatMessage {
            message_type: "textMessage".to_string(),
            id: renderer["id"].as_str()?.to_string(),
            message,
            message_ex,
            timestamp: renderer["timestampUsec"].as_str()?.parse::<i64>().ok()? / 1000,
            datetime: self.format_datetime(renderer["timestampUsec"].as_str()?),
            elapsed_time: None,
            amount_value: None,
            amount_string: None,
            currency: None,
            bg_color: None,
            author,
        })
    }

    // ビューアーエンゲージメントメッセージの解析
    fn parse_viewer_engagement_message(&self, renderer: &Value) -> Option<ChatMessage> {
        let message_runs = self.parse_message_runs(&renderer["message"]["runs"]);

        let message = message_runs
            .iter()
            .filter_map(|run| run.text.clone())
            .collect::<String>();

        Some(ChatMessage {
            message_type: "viewerEngagementMessage".to_string(),
            id: renderer["id"].as_str()?.to_string(),
            message,
            message_ex: Some(message_runs),
            timestamp: renderer["timestampUsec"].as_str()?.parse::<i64>().ok()? / 1000,
            datetime: self.format_datetime(renderer["timestampUsec"].as_str()?),
            elapsed_time: None,
            amount_value: None,
            amount_string: None,
            currency: None,
            bg_color: None,
            author: Author {
                name: "".to_string(),
                channel_id: "".to_string(),
                channel_url: "".to_string(),
                image_url: "".to_string(),
                badge_url: None,
                is_verified: false,
                is_chat_owner: false,
                is_chat_sponsor: false,
                is_chat_moderator: false,
            },
        })
    }

    // スーパーチャットやスーパーステッカーの解析
    fn parse_paid_message(&self, renderer: &Value, message_type: &str) -> Option<ChatMessage> {
        let author = self.parse_author(
            &renderer["authorName"],
            &renderer["authorPhoto"],
            &renderer["authorBadges"],
        )?;
        let message_runs = renderer
            .get("message")
            .and_then(|msg| Some(self.parse_message_runs(&msg["runs"])))
            .unwrap_or_else(Vec::new);

        let message = message_runs
            .iter()
            .filter_map(|run| run.text.clone())
            .collect::<String>();

        let message_ex = Some(message_runs);

        let amount_string = renderer["purchaseAmountText"]["simpleText"]
            .as_str()
            .map(|s| s.to_string());
        let amount_value = amount_string
            .as_ref()
            .and_then(|s| s.replace(",", "").parse::<f64>().ok());

        let bg_color = renderer["headerBackgroundColor"].as_u64().map(|v| v as u32);

        Some(ChatMessage {
            message_type: message_type.to_string(),
            id: renderer["id"].as_str()?.to_string(),
            message,
            message_ex,
            timestamp: renderer["timestampUsec"].as_str()?.parse::<i64>().ok()? / 1000,
            datetime: self.format_datetime(renderer["timestampUsec"].as_str()?),
            elapsed_time: None,
            amount_value,
            amount_string,
            currency: renderer["currency"].as_str().map(|s| s.to_string()),
            bg_color,
            author,
        })
    }

    // メンバーシップメッセージの解析
    fn parse_membership_message(&self, renderer: &Value) -> Option<ChatMessage> {
        let author = self.parse_author(
            &renderer["authorName"],
            &renderer["authorPhoto"],
            &renderer["authorBadges"],
        )?;
        let message_runs = self.parse_message_runs(&renderer["headerSubtext"]["runs"]);

        let message = message_runs
            .iter()
            .filter_map(|run| run.text.clone())
            .collect::<String>();

        let message_ex = Some(message_runs);

        Some(ChatMessage {
            message_type: "newSponsor".to_string(),
            id: renderer["id"].as_str()?.to_string(),
            message,
            message_ex,
            timestamp: renderer["timestampUsec"].as_str()?.parse::<i64>().ok()? / 1000,
            datetime: self.format_datetime(renderer["timestampUsec"].as_str()?),
            elapsed_time: None,
            amount_value: None,
            amount_string: None,
            currency: None,
            bg_color: None,
            author,
        })
    }

    // 投稿者情報の解析
    fn parse_author(
        &self,
        author_name: &Value,
        author_photo: &Value,
        author_badges: &Value,
    ) -> Option<Author> {
        let name = author_name["simpleText"].as_str()?.to_string();
        let image_url = author_photo["thumbnails"][0]["url"].as_str()?.to_string();
        let channel_id = author_photo["thumbnails"][0]["url"]
            .as_str()?
            .split("/")
            .nth(4)?
            .to_string();
        let channel_url = format!("https://www.youtube.com/channel/{}", channel_id);

        // バッジ情報の取得
        let mut badge_url = None;
        let mut is_verified = false;
        let mut is_chat_owner = false;
        let mut is_chat_moderator = false;
        let mut is_chat_sponsor = false;

        if let Some(badges) = author_badges.as_array() {
            for badge in badges {
                if let Some(badge_renderer) = badge.get("liveChatAuthorBadgeRenderer") {
                    // バッジのアイコンURLを取得
                    if let Some(icon) = badge_renderer.get("icon") {
                        badge_url = icon["thumbnails"][0]["url"].as_str().map(|s| s.to_string());
                    } else if let Some(custom_thumbnail) = badge_renderer.get("customThumbnail") {
                        badge_url = custom_thumbnail["thumbnails"][0]["url"]
                            .as_str()
                            .map(|s| s.to_string());
                    }

                    // バッジのラベルを取得
                    if let Some(label) =
                        badge_renderer["accessibility"]["accessibilityData"]["label"].as_str()
                    {
                        let label_lower = label.to_lowercase();
                        // ラベルに応じてフラグを設定
                        if label_lower.contains("verified") || label_lower.contains("認証済み")
                        {
                            is_verified = true;
                        } else if label_lower.contains("moderator")
                            || label_lower.contains("モデレーター")
                        {
                            is_chat_moderator = true;
                        } else if label_lower.contains("owner") || label_lower.contains("所有者")
                        {
                            is_chat_owner = true;
                        } else if label_lower.contains("member") || label_lower.contains("メンバー")
                        {
                            is_chat_sponsor = true;
                        }
                    }
                }
            }
        }

        Some(Author {
            name,
            channel_id,
            channel_url,
            image_url,
            badge_url,
            is_verified,
            is_chat_owner,
            is_chat_sponsor,
            is_chat_moderator,
        })
    }

    // メッセージランの解析
    fn parse_message_runs(&self, runs: &Value) -> Vec<MessageRun> {
        let mut message_runs = Vec::new();
        if let Some(runs_array) = runs.as_array() {
            for run in runs_array {
                if let Some(text) = run.get("text").and_then(|v| v.as_str()) {
                    message_runs.push(MessageRun {
                        text: Some(text.to_string()),
                        emoji: None,
                    });
                } else if let Some(emoji) = run.get("emoji") {
                    let emoji_id = emoji["emojiId"].as_str().unwrap_or("").to_string();
                    let emoji_txt = emoji["shortcuts"][0].as_str().unwrap_or("").to_string();
                    let emoji_url = emoji["image"]["thumbnails"][0]["url"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    message_runs.push(MessageRun {
                        text: None,
                        emoji: Some(Emoji {
                            id: emoji_id,
                            txt: emoji_txt,
                            url: emoji_url,
                        }),
                    });
                }
            }
        }
        message_runs
    }

    // タイムスタンプを日時文字列に変換
    fn format_datetime(&self, timestamp_usec: &str) -> String {
        let timestamp_millis = timestamp_usec.parse::<i64>().unwrap_or(0) / 1000;
        let datetime = Utc.timestamp_millis_opt(timestamp_millis).unwrap();
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}
