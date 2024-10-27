// src/services/minecraft_service.rs

use anyhow::Result;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::net::SocketAddr;

pub struct MinecraftService {
    server_addr: SocketAddr,
}

impl MinecraftService {
    pub fn new(server_addr: &str) -> Result<Self> {
        let addr = server_addr.parse()?;
        Ok(Self {
            server_addr: addr,
        })
    }

    pub async fn send_chat_message(&self, message: &str) -> Result<()> {
        let mut stream = TcpStream::connect(self.server_addr).await?;

        stream.write_all(message.as_bytes()).await?;
        stream.shutdown().await?;
        Ok(())
    }
}
