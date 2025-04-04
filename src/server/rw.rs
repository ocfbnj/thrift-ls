use std::{io, ops::DerefMut, sync::Arc};

use thrift_ls::lsp;

use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

pub struct MessageReader<R> {
    lsp_reader: lsp::MessageReader,
    reader: R,
}

impl<R: AsyncReadExt + Unpin> MessageReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            lsp_reader: lsp::MessageReader::new(),
            reader,
        }
    }
}

impl<R: AsyncReadExt + Unpin> MessageReader<R> {
    pub async fn read_message(&mut self) -> io::Result<lsp::BaseMessage> {
        self.lsp_reader.read_message(&mut self.reader).await
    }
}

pub struct MessageWriter<W> {
    lsp_writer: lsp::MessageWriter,
    writer: Arc<Mutex<W>>,
}

impl<W: AsyncWriteExt + Unpin + Send> MessageWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            lsp_writer: lsp::MessageWriter::new(),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub async fn write_message(&self, message: &impl Serialize) -> io::Result<()> {
        log::debug!(
            "Write Message: {}",
            serde_json::to_string(&message).unwrap_or("<None>".to_string())
        );
        let mut writer = self.writer.lock().await;
        self.lsp_writer
            .write_message(writer.deref_mut(), message)
            .await
    }
}

impl<W: AsyncWriteExt + Unpin + Send> Clone for MessageWriter<W> {
    fn clone(&self) -> Self {
        Self {
            lsp_writer: self.lsp_writer.clone(),
            writer: self.writer.clone(),
        }
    }
}
