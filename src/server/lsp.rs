use std::io;

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use thrift_analyzer::analyzer::base;

// represents request message or notification message
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub method: String,
    pub params: Option<Value>,
}

impl BaseMessage {
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

// represents response message
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseResponse {
    pub jsonrpc: String,
    pub id: Option<i32>,
    pub result: Option<Value>,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<i64>,
    pub client_info: Option<ClientInfo>,
    pub locale: Option<String>,
    pub root_path: Option<String>,
    pub root_uri: Option<String>,
    pub initialization_options: Option<Value>,
    pub capabilities: Option<Value>,
    pub trace: Option<String>,
    pub workspace_folders: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub capabilities: Value,
    pub server_info: Option<ServerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentContentChangeEvent {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidCloseTextDocumentParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u32>,
    pub source: Option<String>,
    pub message: String,
}

impl From<base::Error> for Diagnostic {
    fn from(value: base::Error) -> Self {
        Diagnostic {
            range: value.range.into(),
            severity: Some(1),
            source: Some(env!("CARGO_PKG_NAME").to_string()),
            message: value.message,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl From<base::Range> for Range {
    fn from(value: base::Range) -> Self {
        Range {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl From<base::Position> for Position {
    fn from(value: base::Position) -> Self {
        Position {
            line: value.line as u32 - 1,
            character: value.column as u32 - 1,
        }
    }
}

impl Into<base::Position> for Position {
    fn into(self) -> base::Position {
        base::Position {
            line: self.line as u32 + 1,
            column: self.character as u32 + 1,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokensOptions {
    pub legend: SemanticTokensLegend,
    pub full: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokensLegend {
    pub token_types: Vec<String>,
    pub token_modifiers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokensParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokens {
    pub data: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug)]
pub struct MessageReader {
    buffer: BytesMut,
}

impl MessageReader {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }

    pub async fn read_message<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut R,
    ) -> io::Result<BaseMessage> {
        loop {
            reader.read_buf(&mut self.buffer).await?;

            if let Some(message) = self.try_decode_message()? {
                return Ok(message);
            }
        }
    }

    fn try_decode_message(&mut self) -> io::Result<Option<BaseMessage>> {
        // find the end of the header
        let header_end = match self
            .buffer
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            Some(pos) => pos,
            None => return Ok(None),
        };

        // parse content length from header
        let header = String::from_utf8_lossy(&self.buffer[..header_end]);
        let content_length = match header
            .lines()
            .find(|line| line.starts_with("Content-Length: "))
            .and_then(|line| line["Content-Length: ".len()..].parse::<usize>().ok())
        {
            Some(len) => len,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid Content-Length header",
                ))
            }
        };

        // check if message is complete
        let message_start = header_end + 4;
        if self.buffer.len() < message_start + content_length {
            return Ok(None);
        }

        // extract message and remove it from buffer
        let message = &self.buffer.split_to(message_start + content_length)[message_start..];
        let message = if let Ok(base_message) = serde_json::from_slice::<BaseMessage>(message) {
            base_message
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid message format",
            ));
        };

        Ok(Some(message))
    }
}

#[derive(Debug, Clone)]
pub struct MessageWriter;

impl MessageWriter {
    pub fn new() -> Self {
        Self
    }

    pub async fn write_message<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        message: &impl Serialize,
    ) -> io::Result<()> {
        let message = self.encode_message(message);
        writer.write_all(message.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    fn encode_message(&self, message: &impl Serialize) -> String {
        let content = serde_json::to_string(message).unwrap_or_default();
        format!("Content-Length: {}\r\n\r\n{}", content.len(), content)
    }
}
