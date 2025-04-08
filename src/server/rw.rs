use std::io;

use thrift_ls::lsp;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A reader for LSP messages.
pub struct MessageReader<R> {
    lsp_reader: lsp::MessageReader,
    reader: R,
}

impl<R: AsyncReadExt + Unpin> MessageReader<R> {
    /// Creates a new message reader.
    pub fn new(reader: R) -> Self {
        Self {
            lsp_reader: lsp::MessageReader::new(),
            reader,
        }
    }

    /// Reads a message from the reader.
    pub async fn read_message(&mut self) -> io::Result<lsp::BaseMessage> {
        self.lsp_reader.read_message(&mut self.reader).await
    }
}

/// A writer for LSP messages.
pub struct MessageWriter<W> {
    lsp_writer: lsp::MessageWriter,
    writer: W,
}

impl<W: AsyncWriteExt + Unpin> MessageWriter<W> {
    /// Creates a new message writer.
    pub fn new(writer: W) -> Self {
        Self {
            lsp_writer: lsp::MessageWriter::new(),
            writer,
        }
    }

    /// Writes a message to the writer.
    pub async fn write_message(&mut self, message: &impl Serialize) -> io::Result<()> {
        log::debug!(
            "Write Message: {}",
            serde_json::to_string(&message).unwrap_or("<None>".to_string())
        );
        self.lsp_writer
            .write_message(&mut self.writer, message)
            .await
    }
}
