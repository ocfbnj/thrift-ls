
import * as path from 'path';

import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const serverOptions = getServerOptions(context);
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'thrift' }],
        synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher('**/*.thrift') },
    };

    client = new LanguageClient('thriftLanguageServer', 'Thrift Language Server', serverOptions, clientOptions);
    client.start();
}

function getServerOptions(context: vscode.ExtensionContext): ServerOptions {
    if (context.extensionMode === vscode.ExtensionMode.Production) {
        return getBundleServerOptions(context);
    }

    return getBinaryServerOptions(context);
}

function getBundleServerOptions(context: vscode.ExtensionContext): ServerOptions {
    const serverModule = context.asAbsolutePath(path.join('out', 'server.js'));
    const serverOptions: ServerOptions = {
        run: { module: serverModule, transport: TransportKind.ipc },
        debug: { module: serverModule, transport: TransportKind.ipc, }
    };

    return serverOptions;
}

function getBinaryServerOptions(context: vscode.ExtensionContext): ServerOptions {
    const isWindows = process.platform === 'win32';
    const executableName = isWindows ? 'thrift-ls.exe' : 'thrift-ls';
    const projectRoot = path.join(context.extensionPath, '..', '..');
    const debugPath = path.join(projectRoot, 'target', 'debug', executableName);
    const serverPath = debugPath;

    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio, },
        debug: { command: serverPath, transport: TransportKind.stdio, }
    };

    return serverOptions;
}

export function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
