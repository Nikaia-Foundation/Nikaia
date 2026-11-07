// The Nikaia language client: starts `nikaia lsp` over stdio for every
// `.nika` document and shows what it publishes (the findings of
// `nikaia check`, Part III, `nikaia lsp`). VS Code loads only JavaScript,
// so this file is JavaScript; the server is the compiler's.
"use strict";

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace
    .getConfiguration("nikaia")
    .get("server.path", "nikaia");
  const server = { command, args: ["lsp"] };
  client = new LanguageClient(
    "nikaia",
    "Nikaia",
    { run: server, debug: server },
    { documentSelector: [{ language: "nikaia" }] },
  );
  context.subscriptions.push(client);
  return client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
