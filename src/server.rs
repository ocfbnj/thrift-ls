use std::io;

use thrift_ls::lsp::{BaseMessage, BaseResponse, MessageReader, MessageWriter};

pub struct LanguageServer {
    reader: MessageReader,
    writer: MessageWriter,
}

impl LanguageServer {
    pub fn new() -> Self {
        Self {
            reader: MessageReader::new(),
            writer: MessageWriter::new(),
        }
    }

    pub async fn run(&mut self) -> io::Result<()> {
        log::info!("Language Server is running");
        let mut stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();

        loop {
            let message = self.reader.read_message(&mut stdin).await?;
            log::debug!("Received message: {}", serde_json::to_string(&message)?);
            match message.method.as_str() {
                "initialize" => {
                    let response = self.handle_initialize(message);
                    if let Some(response) = response {
                        log::debug!("Sending response: {}", serde_json::to_string(&response)?);
                        self.writer.write_message(&mut stdout, &response).await?;
                    }
                }
                _ => {
                    log::warn!("Unhandled method: {}", message.method);
                }
            }
        }
    }

    fn handle_initialize(&mut self, message: BaseMessage) -> Option<BaseResponse> {
        Some(BaseResponse {
            id: message.id,
            result: Some(serde_json::json!({
                "capabilities": {
                    "textDocumentSync": 1
                },
                "serverInfo": {
                    "name": "Thrift Language Server",
                    "version": "0.1.0"
                }
            })),
            error: None,
        })
    }
}
