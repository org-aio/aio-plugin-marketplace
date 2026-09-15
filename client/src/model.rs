use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub(super) struct MarketplaceEntry {
    #[serde(default)]
    pub cli: Option<az_tool::ToolManifest>,
    #[serde(default)]
    pub parent_git: Option<String>,
    #[serde(default)]
    pub parent_title: Option<String>,
    pub git: String,
    pub rev: String,
    pub title: String,
    pub summary: String,
    pub license: String,
    pub tags: Vec<String>,
    pub installed: bool,
    pub source_id: Option<String>,
    pub state: Option<PluginState>,
    pub active_revision: Option<String>,
    pub runtime: Option<String>,
    #[serde(default)]
    pub capabilities: MarketplaceCapabilities,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub(super) struct MarketplaceCapabilities {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub filesystem: Vec<String>,
    #[serde(default)]
    pub database: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PluginState {
    Active,
    Disabled,
    Failed,
}

#[derive(Serialize)]
pub(super) struct InstallRequest {
    pub git: String,
    pub rev: Option<String>,
}

impl MarketplaceEntry {
    pub fn state_label(&self) -> &'static str {
        if self.cli.is_some() {
            "本机 CLI"
        } else if !self.installed {
            "未安装"
        } else {
            match self.state {
                Some(PluginState::Active) => "已启用",
                Some(PluginState::Disabled) => "已停用",
                Some(PluginState::Failed) => "失败",
                None => "已安装",
            }
        }
    }
}

pub(super) fn short_revision(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub(super) struct PluginDetailsView {
    pub readme: String,
    pub version: Option<String>,
    pub source_revision: Option<String>,
    pub versions: Vec<PublishedVersion>,
    pub builds: Vec<BuildView>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct PublishedVersion {
    pub revision: String,
    pub version: String,
    pub source_revision: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct BuildView {
    pub id: i64,
    pub source_revision: String,
    pub state: String,
    pub error: Option<String>,
    pub updated_at: String,
}
