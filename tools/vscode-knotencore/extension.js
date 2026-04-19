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
    // Heuristic: Look for knoten_lsp in the workspace target folders, then configuration, then PATH.
    const serverBinary = process.platform === 'win32' ? 'knoten_lsp.exe' : 'knoten_lsp';
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const configPath = vscode.workspace.getConfiguration('knotencore').get('lspPath');
    
    let serverPath = serverBinary; // Default to PATH
    const fs = require('fs');

    let foundInWorkspace = false;
    if (workspaceRoot) {
        const pathsToCheck = [
            path.join(workspaceRoot, 'target', 'release', serverBinary),
            path.join(workspaceRoot, 'target', 'debug', serverBinary),
            path.join(workspaceRoot, 'aether_compiler', 'target', 'release', serverBinary),
            path.join(workspaceRoot, 'aether_compiler', 'target', 'debug', serverBinary)
        ];
        
        for (const p of pathsToCheck) {
            if (fs.existsSync(p)) {
                serverPath = p;
                foundInWorkspace = true;
                console.log(`KnotenCore LSP: Found workspace binary at ${serverPath}`);
                break;
            }
        }
    }

    if (!foundInWorkspace && configPath && fs.existsSync(configPath)) {
        serverPath = configPath;
        console.log(`KnotenCore LSP: Found configured binary at ${serverPath}`);
    } else if (!foundInWorkspace && !configPath) {
        console.log(`KnotenCore LSP: Using PATH binary ${serverPath}`);
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
