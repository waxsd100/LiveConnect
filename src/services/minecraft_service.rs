// src/services/minecraft_service.rs

use anyhow::Result;
use rcon::Connection;
use std::net::TcpStream;
use std::net::SocketAddr;
use tokio::task;

pub struct MinecraftService {
    server_addr: SocketAddr,
    password: String,
}

impl MinecraftService {
    pub fn new(server_addr: &str, password: &str) -> Result<Self> {
        let addr = server_addr.parse()?;
        Ok(Self {
            server_addr: addr,
            password: password.to_string(),
        })
    }

    pub async fn send_chat_message(&self, message: &str) -> Result<()> {
        let server_addr = self.server_addr;
        let password = self.password.clone();
        let message = message.to_string();

        // spawn_blockingを使用して同期コードを非同期で実行
        task::spawn_blocking(move || {
            let stream = TcpStream::connect(server_addr)?;
            let mut connection = Connection::connect(stream, &password)?;

            // /tellrawコマンドの生成
            let tellraw_command = format!(
                r#"tellraw @a {{ "text": "{}" }}"#,
                message.replace("\"", "\\\"")
            );

            // コマンドの送信
            connection.cmd(&tellraw_command)?;
            Ok::<(), anyhow::Error>(())
        })
            .await??;

        Ok(())
    }
}
