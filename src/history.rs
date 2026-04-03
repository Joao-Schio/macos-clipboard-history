use std::collections::VecDeque;
use tokio::sync::oneshot;
use crate::Reader;

pub enum ManagerRequest {
    Add{content : String},
    Retrieve {response_channel : oneshot::Sender<VecDeque<String>>},
}

pub struct HistoryManager {
    history : VecDeque<String>,
    requests : Reader<ManagerRequest>,
    max_size : usize
}


impl HistoryManager {
    pub fn new(requests : Reader<ManagerRequest>) -> Self {
        Self { history: VecDeque::new(), requests, max_size : 200 }
    }

    #[inline(always)]
    fn find_and_remove(&mut self, content: &str) {
        if let Some(idx) = self.history.iter().position(|entry| entry == content) {
            self.history.remove(idx);
        }
    }

    pub async fn start(&mut self) -> tokio::io::Result<()> {
        while let Some(req) = self.requests.recv().await {
            match req {
                ManagerRequest::Add { content } => {
                    self.find_and_remove(&content);
                    self.history.push_front(content);
                    if self.history.len() > self.max_size {
                        let _ = self.history.pop_back();
                    }
                },
                ManagerRequest::Retrieve { response_channel } => {
                    let _ = response_channel.send(self.history.clone());
                }
            }
        }
        Ok(())
    } 
}
