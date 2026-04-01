use arboard::Clipboard;

use crate::{Reader, Writer};


pub enum ClipboardRequest {
    Get {response : Writer<String> },
    Set {content : String}
}

pub struct ClipboardManager {
    clipboard : Clipboard,
    req_channel : Reader<ClipboardRequest>
}

impl ClipboardManager {
    pub fn new(req_channel : Reader<ClipboardRequest>) -> Self {
        let clipboard = Clipboard::new();
        if let Err(e) = clipboard {
            panic!("Can't have a clipboard history program without a clipboard {}", e);
        }
        let clipboard = clipboard.unwrap();
        Self { clipboard, req_channel }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Some(req) = self.req_channel.recv().await {
            match req {
                ClipboardRequest::Get { response } => {
                    let content = self.clipboard.get_text()?;
                    let _ = response.send(content).await?;
                },
                ClipboardRequest::Set { content } => {
                    self.clipboard.set_text(content)?;
                }
            }
        }
        Ok(())
    }
}