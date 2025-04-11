import {
    createConnection,
    ProposedFeatures,
    InitializeParams,
    TextDocumentSyncKind,
    InitializeResult,
    DidOpenTextDocumentParams,
    DidChangeTextDocumentParams,
    SemanticTokensParams,
    SemanticTokens,
    DefinitionParams,
    Location,
    DidCloseTextDocumentParams,
    Diagnostic,
} from 'vscode-languageserver/node';
import { Analyzer } from 'thrift_analyzer';
import { uriToPath, pathToUri, Error, Location as UtilsLocation, toLspDiagnostic, toLspLocation } from './utils';

const connection = createConnection(ProposedFeatures.all);
const analyzer = Analyzer.new();

connection.onInitialize((params: InitializeParams): InitializeResult => {
    console.debug("[Thrift Language Server] onInitialize, params: ", params);
    return {
        capabilities: {
            textDocumentSync: TextDocumentSyncKind.Full,
            semanticTokensProvider: {
                legend: {
                    tokenTypes: analyzer.semantic_token_types(),
                    tokenModifiers: analyzer.semantic_token_modifiers(),
                },
                full: true
            },
            definitionProvider: true,
        }
    }
});

connection.onDidOpenTextDocument((params: DidOpenTextDocumentParams) => {
    console.debug("[Thrift Language Server] onDidOpenTextDocument, params: ", params);
    const path = uriToPath(params.textDocument.uri);
    const content = params.textDocument.text;

    analyzer.sync_document(path, content);
    publishDiagnostics();
});

connection.onDidChangeTextDocument((params: DidChangeTextDocumentParams) => {
    console.debug("[Thrift Language Server] onDidChangeTextDocument, params: ", params);
    const path = uriToPath(params.textDocument.uri);
    const content = params.contentChanges[0].text;
    analyzer.sync_document(path, content);
    publishDiagnostics();
});

connection.onDidCloseTextDocument((params: DidCloseTextDocumentParams) => {
    console.debug("[Thrift Language Server] onDidCloseTextDocument, params: ", params);
    const path = uriToPath(params.textDocument.uri);
    analyzer.remove_document(path);
});

connection.onRequest("textDocument/semanticTokens/full", (params: SemanticTokensParams): SemanticTokens => {
    console.debug("[Thrift Language Server] onRequest: textDocument/semanticTokens/full, params: ", params);
    const path = uriToPath(params.textDocument.uri);
    const result = analyzer.semantic_tokens(path);

    return {
        data: result ? Array.from(result) : []
    };
});

connection.onDefinition((params: DefinitionParams): Location | null => {
    console.debug("[Thrift Language Server] onDefinition, params: ", params);
    const path = uriToPath(params.textDocument.uri);
    const position = params.position;
    const result: UtilsLocation = analyzer.definition(path, position.line + 1, position.character + 1);
    console.debug("[Thrift Language Server] onDefinition, result: ", result);

    return toLspLocation(result);
});

function publishDiagnostics() {
    const errors_map: Map<string, Error[]> = analyzer.errors();
    console.debug("[Thrift Language Server] publishDiagnostics, errors_map: ", errors_map);

    for (const [path, errors] of errors_map) {
        const diagnostics: Diagnostic[] = [];
        for (const error of errors) {
            diagnostics.push(toLspDiagnostic(error));
        }
        connection.sendDiagnostics({ uri: pathToUri(path), diagnostics });
    }
};

connection.listen();
