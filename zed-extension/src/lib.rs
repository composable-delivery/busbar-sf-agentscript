use zed_extension_api as zed;

const BINARY_NAME: &str = "busbar-sf-agentscript-lsp";
const GITHUB_REPO: &str = "composable-delivery/busbar-sf-agentscript";

struct AgentScriptExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for AgentScriptExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() == "agentscript-lsp" {
            let binary_path = self.lsp_binary_path(worktree)?;
            return Ok(zed::Command {
                command: binary_path,
                args: vec![],
                env: Default::default(),
            });
        }
        Err(format!(
            "Unknown language server: {}",
            language_server_id.as_ref()
        ))
    }

    fn context_server_command(
        &mut self,
        context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> zed::Result<zed::Command> {
        Err(format!(
            "Context server not configured: {}",
            context_server_id.as_ref()
        ))
    }
}

impl AgentScriptExtension {
    fn lsp_binary_path(&mut self, worktree: &zed::Worktree) -> zed::Result<String> {
        if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "{BINARY_NAME}-{}",
            match (platform, arch) {
                (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
                (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
                (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc.exe",
                _ => {
                    return Err(format!(
                        "Unsupported platform: {platform:?} {arch:?}"
                    ))
                }
            }
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "Asset not found in release {}: {asset_name}",
                    release.version
                )
            })?;

        let binary_path = format!("server/{}/{BINARY_NAME}", release.version);

        zed::download_file(
            &asset.download_url,
            &binary_path,
            zed::DownloadedFileType::Uncompressed,
        )
        .map_err(|e| format!("Failed to download {BINARY_NAME}: {e}"))?;

        zed::make_file_executable(&binary_path)?;

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

zed::register_extension!(AgentScriptExtension);
