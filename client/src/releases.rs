use super::model::{MarketplaceEntry, PluginDetailsView};
use az_ui_components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;

fn state_label(state: &str) -> &str {
    match state {
        "queued" => "等待构建",
        "building" | "running" => "正在构建",
        "validating" | "verifying" => "正在校验",
        "uploaded" | "publishing" => "正在发布",
        "published" | "succeeded" | "active" => "已发布",
        "failed" => "构建失败",
        "cancelled" => "已取消",
        _ => "正在处理",
    }
}

#[component]
pub(super) fn ReleaseHistory(
    info: PluginDetailsView,
    entry: MarketplaceEntry,
    busy: bool,
    on_action: Callback<(MarketplaceEntry, String)>,
) -> Element {
    rsx! {
        section { class: "extension-browser__versions",
            h2 { "构建与发布" }
            p { class: "admin-meta", "发布记录独立于当前工作区的安装状态。失败的构建不会替换已安装的版本。" }
            if info.builds.is_empty() { p { "暂无自动构建记录" } }
            for build in &info.builds {
                article { class: "workbench-technical",
                    strong { "{state_label(&build.state)}" } p { "{build.updated_at}" }
                    if build.state == "failed" { p { "构建未完成，可展开技术详情查看原因后重试。" } }
                    details { summary { "构建技术详情" }
                        p { code { "{build.source_revision}" } }
                        if let Some(error) = &build.error { pre { "{error}" } }
                    }
                    if build.state == "failed" { Button { size: ButtonSize::Sm, variant: ButtonVariant::Outline, disabled: busy, onclick: { let entry=entry.clone(); let id=build.id; move |_| on_action.call((entry.clone(),format!("retry:{id}"))) }, "重试构建" } }
                }
            }
            h2 { "已发布版本" }
            if info.versions.is_empty() { p { "暂无版本记录" } }
            for version in &info.versions {
                article { class: "workbench-technical",
                    strong { "{version.version}" } p { "{version.created_at}" }
                    if entry.active_revision.as_deref() == Some(&version.revision) { span { class: "admin-status", "data-enabled": true, "当前工作区正在使用" } }
                    details { summary { "版本技术详情" }
                        p { "包版本：" code { "{version.revision}" } }
                        if let Some(source) = &version.source_revision { p { "源代码：" code { "{source}" } } }
                    }
                }
            }
        }
    }
}
