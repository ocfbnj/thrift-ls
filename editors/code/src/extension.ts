import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind, ErrorAction, CloseAction } from 'vscode-languageclient/node';

let languageClient: LanguageClient

export function activate(context: vscode.ExtensionContext) {
    console.log('[Thrift LS] Extension is now activating...');
    console.log(`[Thrift LS] Extension path: ${context.extensionPath}`);

    // get the path to the language server executable
    const serverPath = getServerPath(context);
    if (!serverPath) {
        console.error('[Thrift LS] Failed to find server executable');
        vscode.window.showErrorMessage('Failed to find Thrift Language Server executable');
        return;
    }
    console.log(`[Thrift LS] Found server executable at: ${serverPath}`);

    // start the language server
    startLanguageServer(serverPath);
}

function getServerPath(context: vscode.ExtensionContext): string | null {
    // get the project root directory (parent directory of editors/code)
    const projectRoot = path.join(context.extensionPath, '..', '..');

    // in development environment, the server executable is in target/debug directory
    const debugPath = path.join(projectRoot, 'target', 'debug', 'thrift-ls.exe');
    if (fs.existsSync(debugPath)) {
        return debugPath;
    }

    // in release environment, the server executable is in bin directory
    const releasePath = path.join(projectRoot, 'bin', 'thrift-ls.exe');
    if (fs.existsSync(releasePath)) {
        return releasePath;
    }

    return null;
}

function startLanguageServer(serverPath: string) {
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio,
            args: []
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
            args: []
        }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'thrift' },
            { pattern: '**/*.thrift' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.thrift')
        },
        diagnosticCollectionName: 'thrift-ls',
        outputChannelName: 'Thrift Language Server',
        connectionOptions: {
            maxRestartCount: 3
        },
        errorHandler: {
            closed: () => {
                console.error('[Thrift LS] Connection closed');
                vscode.window.showErrorMessage('Thrift Language Server connection closed');
                return { action: CloseAction.DoNotRestart };
            },
            error: (error: Error, message: any, count: number) => {
                console.error('[Thrift LS] Connection error:', error);
                vscode.window.showErrorMessage(`Thrift Language Server error: ${error.message}`);
                return { action: ErrorAction.Shutdown };
            }
        }
    };

    languageClient = new LanguageClient(
        'thrift-ls',
        'Thrift Language Server',
        serverOptions,
        clientOptions
    );

    languageClient.start().catch(error => {
        console.error('[Thrift LS] Failed to start language client:', error);
        vscode.window.showErrorMessage(`Failed to start Thrift Language Server: ${error.message}`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    console.log('[Thrift LS] Extension is now deactivating...');

    if (!languageClient) {
        return undefined;
    }
    return languageClient.stop();
}
