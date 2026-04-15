use crate::{Writer, clipboard_manager::ClipboardRequest, history::ManagerRequest};



pub struct ContentManager {
    clip_channel : Writer<ClipboardRequest>,
    history_channel : Writer<ManagerRequest>,
    last : String,
}

impl ContentManager {
    pub fn new(clip_channel : Writer<ClipboardRequest>, history_channel : Writer<ManagerRequest>) -> Self {
        Self { clip_channel, history_channel, last: String::new() }
    }

    pub async fn start(&mut self) -> tokio::io::Result<()> {
        loop {
            let (writer, reader) = tokio::sync::oneshot::channel();
            let _ = self.clip_channel.send(
                ClipboardRequest::Get { response: writer }
            ).await;

            if let Ok(candidate) = reader.await {
                if self.last != candidate {
                    self.last = candidate;
                    let _  = self.history_channel.send(
                        ManagerRequest::Add { content: self.last.clone() }
                    ).await;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    }
} 