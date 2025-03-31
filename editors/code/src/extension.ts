import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind, Trace, State, ErrorAction, CloseAction } from 'vscode-languageclient/node';

let languageClient: LanguageClient

export function activate(context: vscode.ExtensionContext) {
    console.log('[Thrift LS] Extension is now activating...');
    console.log(`[Thrift LS] Extension path: ${context.extensionPath}`);

    // 获取语言服务器可执行文件的路径
    const serverPath = getServerPath(context);
    if (!serverPath) {
        console.error('[Thrift LS] Failed to find server executable');
        vscode.window.showErrorMessage('Failed to find Thrift Language Server executable');
        return;
    }
    console.log(`[Thrift LS] Found server executable at: ${serverPath}`);

    // 启动语言服务器
    startLanguageServer(serverPath);
}

function getServerPath(context: vscode.ExtensionContext): string | null {
    // 获取项目根目录（editors/code 的父目录）
    const projectRoot = path.join(context.extensionPath, '..', '..');

    // 在开发环境中，服务器可执行文件位于 target/debug 目录
    const debugPath = path.join(projectRoot, 'target', 'debug', 'thrift-ls.exe');
    if (fs.existsSync(debugPath)) {
        return debugPath;
    }

    // 在发布环境中，服务器可执行文件位于 bin 目录
    const releasePath = path.join(projectRoot, 'bin', 'thrift-ls.exe');
    if (fs.existsSync(releasePath)) {
        return releasePath;
    }

    return null;
}

function startLanguageServer(serverPath: string) {
    console.log('[Thrift LS] Starting language server...');

    // 创建语言客户端
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
        documentSelector: [{ scheme: 'file', language: 'thrift' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.thrift')
        },
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

    console.log('[Thrift LS] Creating language client...');
    languageClient = new LanguageClient(
        'thrift-ls',
        'Thrift Language Server',
        serverOptions,
        clientOptions
    );

    // 添加连接状态监听
    languageClient.onDidChangeState(({ oldState, newState }) => {
        console.log(`[Thrift LS] Client state changed from ${oldState} to ${newState}`);
        if (newState === State.Stopped) {
            console.error('[Thrift LS] Client stopped unexpectedly');
            vscode.window.showErrorMessage('Thrift Language Server stopped unexpectedly');
        }
    });

    // 添加请求和响应的日志处理
    languageClient.onNotification('$/logTrace', (params: any) => {
        console.log(`[Thrift LS Trace] ${JSON.stringify(params, null, 2)}`);
    });

    languageClient.onRequest('$/logTrace', (params: any) => {
        console.log(`[Thrift LS Trace] ${JSON.stringify(params, null, 2)}`);
        return Promise.resolve();
    });

    // 启动客户端
    console.log('[Thrift LS] Starting language client...');
    languageClient.start().then(() => {
        console.log('[Thrift LS] Language client started successfully');
        // 启用详细日志
        languageClient.outputChannel.show();
        languageClient.outputChannel.appendLine('LSP Trace enabled');
    }).catch(error => {
        console.error('[Thrift LS] Failed to start language client:', error);
        vscode.window.showErrorMessage(`Failed to start Thrift Language Server: ${error.message}`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!languageClient) {
        return undefined;
    }
    return languageClient.stop();
}
