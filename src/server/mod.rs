mod io;
mod lsp;

use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use thrift_ls::analyzer::Analyzer;

use io::{MessageReader, MessageWriter};
use lsp::{
    BaseMessage, BaseResponse, DefinitionParams, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    Location, PublishDiagnosticsParams, ResponseError, SemanticTokens, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensParams, ServerInfo,
};

pub struct LanguageServer<R, W> {
    reader: MessageReader<R>,
    writer: MessageWriter<W>,
    analyzer: Analyzer,
    initialized: bool,
}

impl<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin> LanguageServer<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: MessageReader::new(reader),
            writer: MessageWriter::new(writer),
            analyzer: Analyzer::new(),
            initialized: false,
        }
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
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
                    self.did_open(message).await;
                }
                "textDocument/didChange" => {
                    self.did_change(message).await;
                }
                "textDocument/didClose" => {
                    self.did_close(message).await;
                }
                "textDocument/didSave" => {
                    // do nothing
                }
                "textDocument/semanticTokens/full" => {
                    self.semantic_tokens_full(message).await;
                }
                "textDocument/definition" => {
                    self.definition(message).await;
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
                token_types: vec!["type".to_string(), "function".to_string()],
                token_modifiers: vec![],
            },
            full: Some(true),
        };

        let result = InitializeResult {
            capabilities: serde_json::json!({
                "textDocumentSync": 1, // Documents are synced by always sending the full content of the document.
                "semanticTokensProvider": semantic_tokens_options,
                "definitionProvider": true,
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

impl<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin> LanguageServer<R, W> {
    pub async fn did_open(&mut self, message: BaseMessage) {
        let params = match message.params {
            Some(params) => match serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                Ok(params) => params,
                Err(e) => {
                    log::error!("Failed to parse didOpen params: {}", e);
                    return;
                }
            },
            None => {
                log::error!("Missing params in didOpen message");
                return;
            }
        };

        let uri = params.text_document.uri;
        let content = params.text_document.text;

        self.sync_document(&uri, &content).await;
        self.publish_diagnostics().await;
    }

    pub async fn did_change(&mut self, message: BaseMessage) {
        let params = match message.params {
            Some(params) => match serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                Ok(params) => params,
                Err(e) => {
                    log::error!("Failed to parse didChange params: {}", e);
                    return;
                }
            },
            None => {
                log::error!("Missing params in didChange message");
                return;
            }
        };

        let uri = params.text_document.uri;
        let content = match params.content_changes.last() {
            Some(change) => change.text.clone(),
            None => {
                log::warn!("Missing content in didChange message");
                return;
            }
        };

        self.sync_document(&uri, &content).await;
        self.publish_diagnostics().await;
    }

    pub async fn did_close(&mut self, message: BaseMessage) {
        let params = match message.params {
            Some(params) => match serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                Ok(params) => params,
                Err(e) => {
                    log::error!("Failed to parse didClose params: {}", e);
                    return;
                }
            },
            None => {
                log::error!("Missing params in didClose message");
                return;
            }
        };

        self.remove_document(&params.text_document.uri).await;
    }

    pub async fn semantic_tokens_full(&mut self, message: BaseMessage) {
        let params = match message.params {
            Some(params) => match serde_json::from_value::<SemanticTokensParams>(params) {
                Ok(params) => params,
                Err(e) => {
                    log::error!("Failed to parse semantic tokens params: {}", e);
                    return;
                }
            },
            None => {
                log::error!("Missing params in semantic tokens request");
                return;
            }
        };

        let path = match parse_uri_to_path(&params.text_document.uri) {
            Some(path) => path,
            None => return,
        };

        let tokens = self
            .analyzer
            .semantic_tokens(&path)
            .cloned()
            .unwrap_or_default();

        let response = BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: serde_json::to_value(SemanticTokens { data: tokens }).ok(),
            error: None,
        };

        if let Err(e) = self.writer.write_message(&response).await {
            log::error!("Failed to write response: {}", e);
        }
    }

    pub async fn definition(&mut self, message: BaseMessage) {
        let params = match message.params {
            Some(params) => match serde_json::from_value::<DefinitionParams>(params) {
                Ok(params) => params,
                Err(e) => {
                    log::error!("Failed to parse definition params: {}", e);
                    return;
                }
            },
            None => {
                log::error!("Missing params in definition request");
                return;
            }
        };

        let uri = match parse_uri_to_path(&params.text_document.uri) {
            Some(x) => x,
            None => return,
        };

        let location = self
            .analyzer
            .definition(&uri, params.position.into())
            .map(|(path, definition)| (path, definition.identifier().range.clone().into()))
            .map(|(path, range)| Location {
                uri: path_to_uri(&path),
                range,
            });

        let response = BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: serde_json::to_value(location).ok(),
            error: None,
        };

        if let Err(e) = self.writer.write_message(&response).await {
            log::error!("Failed to write response: {}", e);
        }
    }

    async fn sync_document(&mut self, uri: &str, content: &str) {
        let path = match parse_uri_to_path(&uri) {
            Some(x) => x,
            None => return,
        };

        self.analyzer.sync_document(path.clone(), content);
    }

    async fn remove_document(&mut self, uri: &str) {
        let path = match parse_uri_to_path(&uri) {
            Some(x) => x,
            None => return,
        };
        self.analyzer.remove_document(path.clone());
    }

    async fn publish_diagnostics(&mut self) {
        let errors_map = self.analyzer.errors();

        for (path, errors) in errors_map.iter() {
            let mut diagnostics_params = PublishDiagnosticsParams {
                uri: path_to_uri(path),
                diagnostics: Vec::with_capacity(errors.len()),
            };
            for error in errors {
                diagnostics_params.diagnostics.push(error.clone().into());
            }

            let message = BaseMessage {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: "textDocument/publishDiagnostics".to_string(),
                params: serde_json::to_value(diagnostics_params).ok(),
            };
            if let Err(e) = self.writer.write_message(&message).await {
                log::error!("Failed to write diagnostics: {}", e);
            }
        }
    }
}

fn parse_uri_to_path(uri: &str) -> Option<PathBuf> {
    let url = match Url::parse(&uri) {
        Ok(url) => url,
        Err(e) => {
            log::error!("Parse uri failed, err: {}", e);
            return None;
        }
    };
    let path = match url.to_file_path() {
        Ok(path) => path,
        Err(_) => {
            log::error!("Convert url {} to path failed", url);
            return None;
        }
    };
    Some(path)
}

fn path_to_uri(path: &PathBuf) -> String {
    if cfg!(windows) {
        // Windows paths need special handling
        Url::from_file_path(path)
            .unwrap_or_else(|_| {
                // Fallback for UNC paths or other special cases
                let path_str = path.to_string_lossy();
                Url::parse(&format!("file:///{}", path_str.replace('\\', "/")))
                    .unwrap_or_else(|_| Url::parse("file:///").unwrap())
            })
            .to_string()
    } else {
        // Unix paths are simpler
        Url::from_file_path(path)
            .unwrap_or_else(|_| Url::parse("file:///").unwrap())
            .to_string()
    }
}
