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
    CompletionParams,
    CompletionItem,
    CompletionItemKind,
} from 'vscode-languageserver/node';
import { Analyzer } from 'thrift_analyzer';
import { uriToPath, pathToUri, Error, Location as UtilsLocation, toLspDiagnostic, toLspLocation, readFile } from './utils';

const connection = createConnection(ProposedFeatures.all);
const analyzer = Analyzer.new();
analyzer.set_wasm_read_file(readFile);

connection.onInitialize((_params: InitializeParams): InitializeResult => {
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
            completionProvider: {
                resolveProvider: false,
                triggerCharacters: ['.'],
            },
        }
    }
});

connection.onDidOpenTextDocument((params: DidOpenTextDocumentParams) => {
    const path = uriToPath(params.textDocument.uri);
    const content = params.textDocument.text;

    analyzer.sync_document(path, content);
    publishDiagnostics();
});

connection.onDidChangeTextDocument((params: DidChangeTextDocumentParams) => {
    const path = uriToPath(params.textDocument.uri);
    const content = params.contentChanges[0].text;
    analyzer.sync_document(path, content);
    publishDiagnostics();
});

connection.onDidCloseTextDocument((params: DidCloseTextDocumentParams) => {
    const path = uriToPath(params.textDocument.uri);
    analyzer.remove_document(path);
});

connection.onRequest("textDocument/semanticTokens/full", (params: SemanticTokensParams): SemanticTokens => {
    const path = uriToPath(params.textDocument.uri);
    const result = analyzer.semantic_tokens(path);

    return {
        data: result ? Array.from(result) : []
    };
});

connection.onDefinition((params: DefinitionParams): Location | null => {
    const path = uriToPath(params.textDocument.uri);
    const position = params.position;
    const result: UtilsLocation = analyzer.definition(path, position.line + 1, position.character + 1);

    return toLspLocation(result);
});

connection.onCompletion((params: CompletionParams): CompletionItem[] => {
    const path = uriToPath(params.textDocument.uri);
    const position = params.position;
    const types: string[] = analyzer.types_for_completion(path, position.line + 1, position.character + 1);
    let completionItems: CompletionItem[] = types.map((item) => ({ label: item, kind: CompletionItemKind.Struct }));

    if (params.context?.triggerCharacter != '.') {
        const includes: string[] = analyzer.includes_for_completion(path, position.line + 1, position.character + 1);
        const includeItems: CompletionItem[] = includes.map((item) => ({ label: item, kind: CompletionItemKind.Module }));
        completionItems = completionItems.concat(includeItems);

        const keywords: string[] = analyzer.keywords_for_completion();
        const keywordItems: CompletionItem[] = keywords.map((item) => ({ label: item, kind: CompletionItemKind.Keyword }));
        completionItems = completionItems.concat(keywordItems);
    }

    return completionItems;
});

function publishDiagnostics() {
    const errors_map: Map<string, Error[]> = analyzer.errors();

    for (const [path, errors] of errors_map) {
        const diagnostics: Diagnostic[] = [];
        for (const error of errors) {
            diagnostics.push(toLspDiagnostic(error));
        }
        connection.sendDiagnostics({ uri: pathToUri(path), diagnostics });
    }
};

connection.listen();
