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
    #[serde(default)]
    pub menu_hidden: bool,
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
    Suspended,
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
            if self.installed {
                "已安装到设备"
            } else {
                "设备 CLI"
            }
        } else if !self.installed {
            "未安装"
        } else {
            match self.state {
                Some(PluginState::Active) => "已启用",
                Some(PluginState::Suspended) => "运行环境已暂停",
                Some(PluginState::Disabled) => "已停用",
                Some(PluginState::Failed) => "失败",
                None => "已安装",
            }
        }
    }
}

/// 目录中一条页面入口；重命名面板据此列出可覆盖的导航元素。
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct CatalogPage {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub scene: Option<CatalogScene>,
    #[serde(default)]
    pub menu_path: Vec<CatalogMenuGroup>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct CatalogScene {
    pub id: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct CatalogMenuGroup {
    pub id: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct CatalogAccountItem {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub page_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct CatalogPlugin {
    #[serde(default)]
    pub source_id: String,
    #[serde(default)]
    pub git: String,
}

/// 当前租户已经生效的标签覆盖；键是稳定标识，可据此恢复默认。
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct NavigationLabel {
    pub kind: String,
    pub key: String,
    pub label: String,
}

/// 重命名面板需要的目录子集；完整目录仍用于权限和挂载。
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub(super) struct NavigationCatalog {
    #[serde(default)]
    pub pages: Vec<CatalogPage>,
    #[serde(default)]
    pub account_items: Vec<CatalogAccountItem>,
    #[serde(default)]
    pub plugins: Vec<CatalogPlugin>,
    #[serde(default)]
    pub navigation_labels: Vec<NavigationLabel>,
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
