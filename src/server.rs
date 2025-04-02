use std::io;

use thrift_ls::{
    lsp::{
        BaseMessage, BaseResponse, Diagnostic, DidChangeTextDocumentParams,
        DidOpenTextDocumentParams, InitializeParams, InitializeResult, MessageReader,
        MessageWriter, PublishDiagnosticsParams, ResponseError, ServerInfo,
    },
    parser::Parser,
    scanner::FileInput,
};
use tokio::io::AsyncWriteExt;
use url::Url;

pub struct LanguageServer {
    reader: MessageReader,
    writer: MessageWriter,
    initialized: bool,
}

impl LanguageServer {
    pub fn new() -> Self {
        Self {
            reader: MessageReader::new(),
            writer: MessageWriter::new(),
            initialized: false,
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
                "textDocument/didOpen" => {
                    self.handle_test_document_did_open(message, &mut stdout)
                        .await?;
                }
                "textDocument/didChange" => {
                    self.handle_test_document_did_change(message, &mut stdout)
                        .await?;
                }
                _ => {
                    log::warn!("Unhandled method: {}", message.method);
                }
            }
        }
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
        let result = InitializeResult {
            capabilities: serde_json::json!({
                "textDocumentSync": 1,
            }),
            server_info: Some(ServerInfo {
                name: "Thrift Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        };

        Some(BaseResponse {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            result: serde_json::to_value(result).ok(),
            error: None,
        })
    }

    async fn handle_test_document_did_open<W: AsyncWriteExt + Unpin>(
        &mut self,
        message: BaseMessage,
        writer: &mut W,
    ) -> io::Result<()> {
        let params = serde_json::from_value::<DidOpenTextDocumentParams>(
            message.params.ok_or(io::ErrorKind::InvalidData)?,
        )?;
        let uri = params.text_document.uri;

        // convert uri to path
        let path = Url::parse(&uri)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse URI: {}", e),
                )
            })?
            .to_file_path()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid file path in URI: {}", uri),
                )
            })?;

        let mut parser = Parser::new(FileInput::new(&path));
        parser.parse();
        if parser.errors().len() <= 0 {
            return Ok(());
        }

        let mut diagnostics_params = PublishDiagnosticsParams {
            uri,
            diagnostics: vec![],
        };

        for err in parser.errors() {
            let diagnostic = Diagnostic {
                range: err.range.clone().into(),
                severity: Some(1),
                source: Some("thrift".to_string()),
                message: err.message.to_string(),
            };

            diagnostics_params.diagnostics.push(diagnostic);
        }

        let message = BaseMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "textDocument/publishDiagnostics".to_string(),
            params: serde_json::to_value(diagnostics_params).ok(),
        };
        log::debug!(
            "Write Message: {}",
            serde_json::to_string(&message).unwrap()
        );
        self.writer.write_message(writer, &message).await?;

        Ok(())
    }

    async fn handle_test_document_did_change<W: AsyncWriteExt + Unpin>(
        &mut self,
        message: BaseMessage,
        writer: &mut W,
    ) -> io::Result<()> {
        let params = serde_json::from_value::<DidChangeTextDocumentParams>(
            message.params.ok_or(io::ErrorKind::InvalidData)?,
        )?;
        let uri = params.text_document.uri;

        // convert uri to path
        let path = Url::parse(&uri)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse URI: {}", e),
                )
            })?
            .to_file_path()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid file path in URI: {}", uri),
                )
            })?;

        let mut parser = Parser::new(FileInput::new_with_string(
            &path,
            &params.content_changes[0].text,
        ));
        parser.parse();
        if parser.errors().len() <= 0 {
            return Ok(());
        }

        let mut diagnostics_params = PublishDiagnosticsParams {
            uri,
            diagnostics: vec![],
        };

        for err in parser.errors() {
            let diagnostic = Diagnostic {
                range: err.range.clone().into(),
                severity: Some(1),
                source: Some("thrift".to_string()),
                message: err.message.to_string(),
            };

            diagnostics_params.diagnostics.push(diagnostic);
        }

        let message = BaseMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "textDocument/publishDiagnostics".to_string(),
            params: serde_json::to_value(diagnostics_params).ok(),
        };
        log::debug!(
            "Write Message: {}",
            serde_json::to_string(&message).unwrap()
        );
        self.writer.write_message(writer, &message).await?;

        Ok(())
    }
}
