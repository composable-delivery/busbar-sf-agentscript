# VS Code Extension Testing Guide

## Setup Complete ✅

All components are now set up to test the AgentScript VS Code extension:

### What's Been Done

1. ✅ **Rust LSP Binary Built**: `/Users/jasonlantz/dev/0-busbar/sf-agentscript/target/debug/agentscript-lsp`
2. ✅ **TypeScript Compiled**: Extension code compiled to `./out`
3. ✅ **Dependencies Installed**: npm packages ready
4. ✅ **Launch Configuration Created**: `.vscode/launch.json` configured

---

## Testing the Extension

### Option 1: Run in Debug Mode (Recommended)

1. Open the extension folder in VS Code:
   ```bash
   code /Users/jasonlantz/dev/0-busbar/sf-agentscript/packages/vscode-agentscript
   ```

2. Press `F5` or go to **Run > Start Debugging**

3. Select **"Run Extension"** from the debug dropdown

4. A new VS Code window will open with the extension loaded

5. Create or open a `.agent` file to test the language support

### Option 2: Manual Setup in VS Code

If you prefer to test in your existing VS Code window:

1. Open the workspace root in VS Code
2. Install the "Extension Host" VS Code debugger if prompted
3. Navigate to the Run and Debug panel (Ctrl+Shift+D / Cmd+Shift+D)
4. Run the "Run Extension" configuration

---

## Testing Features

Once the extension is running in debug mode:

- **Create a new file**: `test.agent`
- **Syntax highlighting** should activate automatically
- **Language server** will connect via the LSP binary
- **Diagnostics** will appear as you type

## Troubleshooting

### LSP Binary Path Issue
If the language server doesn't start, verify the binary path in:
- `src/extension.ts` line 22

Current path: `/Users/jasonlantz/dev/0-busbar/sf-agentscript/target/debug/agentscript-lsp`

### Recompile After Changes
When editing TypeScript:
```bash
cd packages/vscode-agentscript
npm run compile
```

Or use the watch mode:
```bash
npm run watch
```

### Watch Mode for Development
In one terminal:
```bash
cd packages/vscode-agentscript
npm run watch
```

Then press `F5` to debug in another VS Code window.

---

## Next Steps

- [ ] Test syntax highlighting on `.agent` files
- [ ] Verify language server diagnostics
- [ ] Test auto-completion if implemented
- [ ] Check hover information
- [ ] Validate error reporting

---

**Ready to test!** Press `F5` in VS Code to start debugging.
