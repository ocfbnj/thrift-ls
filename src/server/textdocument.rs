use std::{path::PathBuf, sync::Arc};

use tokio::{io::AsyncWriteExt, sync::Mutex};
use url::Url;

use thrift_ls::{
    analyzer::Analyzer,
    lsp::{
        BaseMessage, BaseResponse, Diagnostic, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, PublishDiagnosticsParams,
        SemanticTokens, SemanticTokensParams,
    },
};

use super::rw::MessageWriter;

pub async fn did_open<W: AsyncWriteExt + Unpin + Send>(
    message: BaseMessage,
    writer: MessageWriter<W>,
    analyzer: Arc<Mutex<Analyzer>>,
) {
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

    let content = params.text_document.text;

    diagnostics(writer, &params.text_document.uri, &content, analyzer).await;
}

pub async fn did_change<W: AsyncWriteExt + Unpin + Send>(
    message: BaseMessage,
    writer: MessageWriter<W>,
    analyzer: Arc<Mutex<Analyzer>>,
) {
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

    let content = match params.content_changes.last() {
        Some(change) => change.text.clone(),
        None => {
            log::warn!("Missing content in didChange message");
            return;
        }
    };

    diagnostics(writer, &params.text_document.uri, &content, analyzer).await;
}

pub async fn did_close<W: AsyncWriteExt + Unpin + Send>(
    message: BaseMessage,
    _writer: MessageWriter<W>,
    _analyzer: Arc<Mutex<Analyzer>>,
) {
    let _params = match message.params {
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
}

pub async fn semantic_tokens_full<W: AsyncWriteExt + Unpin + Send>(
    message: BaseMessage,
    writer: MessageWriter<W>,
    analyzer: Arc<Mutex<Analyzer>>,
) {
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
        None => {
            return;
        }
    };

    let analyzer = analyzer.lock().await;
    let tokens = analyzer.semantic_tokens(&path).cloned().unwrap_or_default();

    let response = BaseResponse {
        jsonrpc: "2.0".to_string(),
        id: message.id,
        result: serde_json::to_value(SemanticTokens { data: tokens }).ok(),
        error: None,
    };

    if let Err(e) = writer.write_message(&response).await {
        log::error!("Failed to write response: {}", e);
    }
}

async fn diagnostics<W: AsyncWriteExt + Unpin + Send>(
    writer: MessageWriter<W>,
    uri: &str,
    content: &str,
    analyzer: Arc<Mutex<Analyzer>>,
) {
    let path = match parse_uri_to_path(&uri) {
        Some(x) => x,
        None => return,
    };

    let mut analyzer = analyzer.lock().await;
    analyzer.sync_document(path.clone(), content);
    analyzer.analyze();
    let errors_map = analyzer.errors();

    for (path, errors) in errors_map.iter() {
        let mut diagnostics_params = PublishDiagnosticsParams {
            uri: path_to_uri(path),
            diagnostics: Vec::with_capacity(errors.len()),
        };
        for err in errors {
            let diagnostic = Diagnostic {
                range: err.range.clone().into(),
                severity: Some(1),
                source: Some(env!("CARGO_PKG_NAME").to_string()),
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
        if let Err(e) = writer.write_message(&message).await {
            log::error!("Failed to write diagnostics: {}", e);
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
