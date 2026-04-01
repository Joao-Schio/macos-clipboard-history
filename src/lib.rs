pub mod history;
pub mod clipboard_manager;
pub mod content_manager;

pub type Writer<T> = tokio::sync::mpsc::Sender<T>;
pub type Reader<T> = tokio::sync::mpsc::Receiver<T>;