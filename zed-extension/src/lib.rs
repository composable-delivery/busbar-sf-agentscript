use zed_extension_api as zed;

const NODE_PATH: &str = "/Users/jasonlantz/.nvm/versions/node/v22.11.0/bin/node";

struct AgentScriptExtension;

impl zed::Extension for AgentScriptExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() == "agentscript-lsp" {
            let path = "/Users/jasonlantz/dev/0-busbar/sf-agentscript/target/debug/agentscript-lsp";
            return Ok(zed::Command {
                command: path.to_string(),
                args: vec![],
                env: Default::default(),
            });
        }
        Err(format!("Unknown language server: {}", language_server_id.as_ref()))
    }

    fn context_server_command(
        &mut self,
        context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> zed::Result<zed::Command> {
        if context_server_id.as_ref() == "aslab-mcp" {
            let path = "/Users/jasonlantz/dev/0-busbar/sf-agentscript/aslab-mcp/dist/index.js";
            return Ok(zed::Command {
                command: NODE_PATH.to_string(),
                args: vec![path.to_string()],
                env: Default::default(),
            });
        }
        Err(format!("Unknown context server: {}", context_server_id.as_ref()))
    }
}

zed::register_extension!(AgentScriptExtension);
