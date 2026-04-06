// Minimal extension entry point.
// This extension is grammar-only (no Language Server Protocol for Phase 1).
// All syntax highlighting is handled declaratively via TextMate grammars.

const vscode = require('vscode');

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    console.log('KnotenCore Language Extension activated.');
    // Phase 2 will introduce a Language Server Protocol (LSP) client here
    // for diagnostics, hover documentation, and auto-completion.
}

function deactivate() {}

module.exports = { activate, deactivate };
