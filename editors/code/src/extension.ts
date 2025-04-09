import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import which from 'which';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind, ErrorAction, CloseAction } from 'vscode-languageclient/node';

let languageClient: LanguageClient

export function activate(context: vscode.ExtensionContext) {
    console.log('[Thrift LS] Extension is now activating...');
    console.log(`[Thrift LS] Extension path: ${context.extensionPath}`);

    // get the path to the language server executable
    getServerPath(context).then(serverPath => {
        if (!serverPath) {
            console.error('[Thrift LS] Failed to find server executable');
            vscode.window.showErrorMessage('Failed to find Thrift Language Server executable');
            return;
        }
        console.log(`[Thrift LS] Found server executable at: ${serverPath}`);

        // start the language server
        startLanguageServer(serverPath);
    });
}

async function getServerPath(context: vscode.ExtensionContext): Promise<string | null> {
    const isWindows = process.platform === 'win32';
    const executableName = isWindows ? 'thrift-ls.exe' : 'thrift-ls';
    const isDevelopment = context.extensionMode === vscode.ExtensionMode.Development;

    // get the project root directory (parent directory of editors/code)
    const projectRoot = path.join(context.extensionPath, '..', '..');

    // in development environment, check target/debug directory first
    if (isDevelopment) {
        const debugPath = path.join(projectRoot, 'target', 'debug', executableName);
        if (fs.existsSync(debugPath)) {
            return debugPath;
        }
        console.log('[Thrift LS] Debug build not found:', debugPath);
    }

    // in release environment or if debug build not found, try to find the executable in PATH
    try {
        const systemPath = await which(executableName);
        if (systemPath) {
            return systemPath;
        }
    } catch {
        console.log('[Thrift LS] Executable not found in PATH');
    }

    // try to install using cargo if in release mode or debug build not found
    try {
        const cargoPath = await which('cargo');
        if (cargoPath) {
            vscode.window.showInformationMessage('Installing Thrift Language Server...');

            const cp = require('child_process');
            await new Promise<void>((resolve, reject) => {
                cp.exec('cargo install thrift-ls', (error: Error | null) => {
                    if (error) {
                        reject(error);
                    } else {
                        resolve();
                    }
                });
            });

            try {
                const installedPath = await which(executableName);
                if (installedPath) {
                    return installedPath;
                }
            } catch {
                console.log('[Thrift LS] Installation succeeded but executable not found in PATH');
            }
        }
    } catch {
        vscode.window.showErrorMessage(
            'Rust is not installed. Please install Rust to use the Thrift Language Server. ' +
            'Visit https://rustup.rs/ for installation instructions.'
        );
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
