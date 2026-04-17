const vscode = require('vscode');
const path = require('path');
const { LanguageClient, SettingMonitor } = require('vscode-languageclient/node');

let client;

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    console.log('KnotenCore Language Extension: Activating Phase 2 (LSP)...');

    // 1. Determine the path to the LSP server binary.
    // Heuristic: Look for knoten_lsp in the workspace target folders or expect it in PATH.
    const serverBinary = process.platform === 'win32' ? 'knoten_lsp.exe' : 'knoten_lsp';
    
    // For local development, we prefer the target/debug or target/release folder.
    // In a production extension, we might bundle it or ask for a path in settings.
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    let serverPath = serverBinary; // Default to PATH

    if (workspaceRoot) {
        const debugPath = path.join(workspaceRoot, 'aether_compiler', 'target', 'debug', serverBinary);
        const releasePath = path.join(workspaceRoot, 'aether_compiler', 'target', 'release', serverBinary);
        
        const fs = require('fs');
        if (fs.existsSync(debugPath)) {
            serverPath = debugPath;
            console.log(`KnotenCore LSP: Found debug binary at ${serverPath}`);
        } else if (fs.existsSync(releasePath)) {
            serverPath = releasePath;
            console.log(`KnotenCore LSP: Found release binary at ${serverPath}`);
        }
    }

    // 2. Configure Server Options
    const serverOptions = {
        run: { command: serverPath, args: ["--docs", workspaceRoot] },
        debug: { command: serverPath, args: ["--docs", workspaceRoot] }
    };

    // 3. Configure Client Options
    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'nod' },
            { scheme: 'file', language: 'knoten' }
        ],
        synchronize: {
            // Notify the server about file changes to '.nod' and '.knoten' files contained in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{nod,knoten}')
        }
    };

    // 4. Create and start the client
    client = new LanguageClient(
        'knoten-lsp',
        'KnotenCore Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client. This will also launch the server
    client.start();
    console.log('KnotenCore LSP client started.');
}

function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

module.exports = { activate, deactivate };
