mod rw;
mod textdocument;

use std::{io, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use rw::{MessageReader, MessageWriter};
use thrift_ls::{
    analyzer::Analyzer,
    lsp::{
        BaseMessage, BaseResponse, InitializeParams, InitializeResult, ResponseError,
        SemanticTokensLegend, SemanticTokensOptions, ServerInfo,
    },
};

pub struct LanguageServer<R, W> {
    reader: MessageReader<R>,
    writer: MessageWriter<W>,
    initialized: bool,
    analyzer: Arc<Mutex<Analyzer>>,
}

impl<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin + Send + 'static> LanguageServer<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: MessageReader::new(reader),
            writer: MessageWriter::new(writer),
            initialized: false,
            analyzer: Arc::new(Mutex::new(Analyzer::new())),
        }
    }

    pub async fn run(&mut self) -> io::Result<()> {
        log::debug!("Language Server is running");

        loop {
            let message = self.reader.read_message().await?;
            log::debug!(
                "Received message: {}",
                serde_json::to_string(&message).unwrap_or("<None>".to_string())
            );

            match message.method.as_str() {
                "initialize" => {
                    if let Some(response) = self.handle_initialize(message) {
                        self.writer.write_message(&response).await?;
                    }
                }
                "initialized" => {
                    // do nothing
                }
                "shutdown" => {
                    if let Some(response) = self.handle_shutdown(message) {
                        self.writer.write_message(&response).await?;
                    }
                }
                "exit" => {
                    break;
                }
                "textDocument/didOpen" => {
                    let writer = self.writer.clone();
                    let analyzer = self.analyzer.clone();
                    tokio::spawn(async move {
                        textdocument::did_open(message, writer, analyzer).await;
                    });
                }
                "textDocument/didChange" => {
                    let writer = self.writer.clone();
                    let analyzer = self.analyzer.clone();
                    tokio::spawn(async move {
                        textdocument::did_change(message, writer, analyzer).await;
                    });
                }
                "textDocument/didClose" => {
                    let writer = self.writer.clone();
                    let analyzer = self.analyzer.clone();
                    tokio::spawn(async move {
                        textdocument::did_close(message, writer, analyzer).await;
                    });
                }
                "textDocument/didSave" => {
                    // do nothing
                }
                "textDocument/semanticTokens/full" => {
                    let writer = self.writer.clone();
                    let analyzer = self.analyzer.clone();
                    tokio::spawn(async move {
                        textdocument::semantic_tokens_full(message, writer, analyzer).await;
                    });
                }
                method => {
                    if method.starts_with("$/") {
                        if !message.is_notification() {
                            if let Some(response) = self.handle_method_not_found(message) {
                                self.writer.write_message(&response).await?;
                            }
                        }

                        continue;
                    }

                    if message.is_notification() {
                        log::warn!("Unhandled notification: {}", method);
                    } else {
                        log::warn!("Unhandled request: {}", method);
                    }
                }
            }
        }

        log::debug!("Language Server is stopped");
        Ok(())
    }

    fn handle_initialize(&mut self, message: BaseMessage) -> Option<BaseResponse> {
        let _params = serde_json::from_value::<InitializeParams>(message.params?).ok()?;
        if self.initialized {
            return Some(BaseResponse {
                jsonrpc: "2.0".to_string(),
                id: message.id,
                result: None,
                error: Some(ResponseError {
                    code: -32002,
                    message: "Server already initialized".to_string(),
                    data: None,
                }),
            });
        }

        self.initialized = true;

        let semantic_tokens_options = SemanticTokensOptions {
            legend: SemanticTokensLegend {
                token_types: vec!["type".to_string()],
                token_modifiers: vec![],
            },
            full: Some(true),
        };

        let result = InitializeResult {
            capabilities: serde_json::json!({
                "textDocumentSync": 1, // Documents are synced by always sending the full content of the document.
                "semanticTokensProvider": semantic_tokens_options,
            }),
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        };

        Some(BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: serde_json::to_value(result).ok(),
            error: None,
        })
    }

    fn handle_shutdown(&mut self, message: BaseMessage) -> Option<BaseResponse> {
        self.initialized = false;
        Some(BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: None,
            error: None,
        })
    }

    fn handle_method_not_found(&self, message: BaseMessage) -> Option<BaseResponse> {
        Some(BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: None,
            error: Some(ResponseError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        })
    }
}
