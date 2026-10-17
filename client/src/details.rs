use super::{
    http,
    model::{MarketplaceEntry, PluginDetailsView, PluginState, short_revision},
};
use az_ui_components::{
    admin::StatusMessage,
    button::{Button, ButtonSize, ButtonVariant},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{
    ArrowLeft, Download, Eye, EyeOff, Package, Pause, Play, RotateCcw, Settings, Trash2,
};

#[component]
pub(super) fn PluginDetails(
    entry: MarketplaceEntry,
    entries: Vec<MarketplaceEntry>,
    busy: bool,
    can_manage: bool,
    refresh: u64,
    on_action: Callback<(MarketplaceEntry, String)>,
    on_back: Callback<()>,
) -> Element {
    let settings_git = entry.git.clone();
    let configuration = use_resource(use_reactive!(|settings_git, refresh| async move {
        let git = settings_git;
        let _ = refresh;
        let catalog: serde_json::Value = http::get("/api/runtime/catalog").await?;
        let source = catalog["plugins"]
            .as_array()
            .and_then(|plugins| plugins.iter().find(|plugin| plugin["git"] == git))
            .and_then(|plugin| plugin["source_id"].as_str());
        Ok::<_, String>(
            catalog["plugin_settings"]
                .as_array()
                .and_then(|pages| {
                    pages
                        .iter()
                        .find(|page| page["source_id"].as_str() == source)
                })
                .and_then(|page| page["page_id"].as_str())
                .map(str::to_owned),
        )
    }));
    let settings_page = configuration
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .cloned()
        .flatten();
    let navigation = use_context::<super::navigation::Navigation>();
    let mut interaction_error = use_signal(|| None::<String>);
    let interaction = use_callback(
        move |(operation, parameters): (&'static str, serde_json::Value)| {
            interaction_error.set(None);
            spawn(async move {
                if let Err(error) = http::interaction(operation, parameters).await {
                    interaction_error.set(Some(error));
                }
            });
        },
    );
    let mut retained = use_signal(|| None::<PluginDetailsView>);
    let details = use_resource(use_reactive!(|entry, refresh| async move {
        let _ = refresh;
        http::get::<PluginDetailsView>(&format!("/api/runtime/marketplace/{}/details", entry.rev))
            .await
    }));
    use_effect(move || {
        if let Some(Ok(value)) = details.read().as_ref() {
            retained.set(Some(value.clone()));
        }
    });
    let base = entry.git.trim_end_matches(".git");
    let publisher = base
        .strip_prefix("https://github.com/")
        .and_then(|s| s.split('/').next())
        .unwrap_or("发布者");
    let information = retained();
    let parent = entry
        .parent_git
        .as_ref()
        .and_then(|git| entries.iter().find(|e| &e.git == git));
    let parent_ready = entry.parent_git.is_none()
        || parent.is_some_and(|p| p.installed && p.state == Some(PluginState::Active));
    let children = entries
        .iter()
        .filter(|e| e.parent_git.as_deref() == Some(&entry.git))
        .cloned()
        .collect::<Vec<_>>();
    let source = information
        .as_ref()
        .and_then(|d| d.source_revision.as_deref())
        .unwrap_or(&entry.rev);
    let link_base = format!("{base}/blob/{source}/");
    let image_base = format!("/api/runtime/marketplace/{}/images/", entry.rev);
    rsx! {
        Button { class: "extension-browser__back", variant: ButtonVariant::Ghost, onclick: move |_| on_back.call(()), ArrowLeft {} "插件列表" }
        header { class: "extension-browser__heading", Package {} div {
            h1 { "{entry.title}" }
            p { "{publisher} · {information.as_ref().and_then(|d|d.version.as_deref()).unwrap_or_else(||short_revision(&entry.rev))} · {entry.license}" }
            p { "{entry.summary}" }
        } }
        if let Some(message) = interaction_error() { StatusMessage { error:true,message } }
        div { class: "extension-browser__actions",
            if can_manage {
                Button { variant: ButtonVariant::Outline, disabled: busy,
                    onclick: { let entry = entry.clone(); move |_| on_action.call((entry.clone(), "remove".into())) },
                    Trash2 {} "从市场删除"
                }
            }
            if entry.state == Some(PluginState::Active) {
                if let Some(page_id) = settings_page {
                    Button { variant: ButtonVariant::Outline, onclick: move |_| {
                        interaction.call(("settings",serde_json::json!({"page_id":page_id})));
                    }, Settings {} "插件设置" }
                }
            }

            if !entry.installed { Button { disabled: busy || !parent_ready, onclick: { let entry=entry.clone();move |_| on_action.call((entry.clone(),"install".into())) }, Download {} "安装" } }
            else {
                span { class: "admin-status", "data-enabled": entry.state == Some(PluginState::Active), "工作区：{entry.state_label()}" }
                if !matches!(entry.state, Some(PluginState::Active | PluginState::Suspended)) { Button { disabled: busy || !parent_ready, onclick: { let entry=entry.clone();move |_| on_action.call((entry.clone(),"enable".into())) }, Play {} "启用" } }
            }
            if entry.installed && entry.active_revision.as_deref() != Some(&entry.rev) {
                Button { disabled:busy || !parent_ready, onclick:{let entry=entry.clone();move |_|on_action.call((entry.clone(),"install".into()))}, Download {} "更新到最新版本" }
            }
            if entry.installed { details { class: "extension-browser__menu",
                summary { title: "管理插件", aria_label: "管理插件", Settings {} }
                div { role: "menu", aria_label: "插件操作",
                    button { role: "menuitem", disabled: busy, onclick: { let entry=entry.clone(); move |_| on_action.call((entry.clone(), if entry.menu_hidden {"show-menu"} else {"hide-menu"}.into())) },
                        if entry.menu_hidden { Eye {} "显示菜单" } else { EyeOff {} "隐藏菜单" }
                    }
                    for (action,label) in if matches!(entry.state, Some(PluginState::Active | PluginState::Suspended)) { vec![("disable","停用"),("rollback","回退版本"),("uninstall","卸载")] } else { vec![("rollback","回退版本"),("uninstall","卸载")] } {
                        button { role: "menuitem", disabled: busy, onclick: { let entry=entry.clone();move |_| on_action.call((entry.clone(),action.into())) },
                            match action { "disable"=>rsx!{Pause{}},"enable"=>rsx!{Play{}},"rollback"=>rsx!{RotateCcw{}},_=>rsx!{Trash2{}} } "{label}"
                        }
                    }
                    button { role:"menuitem", onclick: {let revision=entry.rev.clone(); move |_| interaction.call(("download",serde_json::json!({"revision":revision})))}, Download {} "下载插件包" }
                }
            } }
        }
        if entry.state == Some(PluginState::Suspended) {
            p { class: "admin-meta", "运行环境暂不可用，已安装版本和数据保留。联系部署管理员恢复运行环境后，插件将继续运行。" }
        } else if entry.state == Some(PluginState::Active) { p { class: "admin-meta", "租户成员自动获得插件声明的业务权限；宿主管理权限仍按角色授权。" } }
        if entry.menu_hidden { p { class: "admin-meta", "菜单已隐藏，可在管理插件中恢复显示。" } }
        if entry.parent_git.is_some() {
            p { class: "admin-meta", "父插件：{parent.map(|p|p.title.as_str()).or(entry.parent_title.as_deref()).unwrap_or(\"尚未发布\")}" }
            if !parent_ready { if let Some(parent) = parent {
                if parent.state == Some(PluginState::Suspended) { p { class: "admin-meta", "父插件运行环境已暂停。" } }
                else { Button { disabled: busy, variant: ButtonVariant::Outline, onclick: { let parent=parent.clone(); move |_| on_action.call((parent.clone(),if parent.installed {"enable"}else{"install"}.into())) }, Download {} "安装并启用父插件" } }
            } }
        }
        if !children.is_empty() {
            section { class: "extension-browser__optional", aria_label: "可选子插件",
                h2 { "可选子插件" }
                for child in children { div { class: "extension-browser__child-action", strong { "{child.title}" } span { "{child.state_label()}" }
                    if !child.installed { Button { size: ButtonSize::Sm, disabled: busy || !entry.installed || entry.state != Some(PluginState::Active), onclick: { let child=child.clone(); move |_| on_action.call((child.clone(),"install".into())) }, Download {} "安装 {child.title}" } }
                } }
            }
        }
        nav { class: "extension-browser__tabs", role: "tablist", aria_label: "插件详情分类",
            for (id,label) in [("details","介绍"),("versions","版本与发布"),("permissions","权限")] { button { role: "tab", "aria-selected": (navigation.view)().tab==id, onclick: move |_| navigation.change(|view|view.tab=id.into()), "{label}" } }
        }
        div { role: "tabpanel",
            match (navigation.view)().tab.as_str() {
                "permissions" => rsx! { dl { class: "admin-details", dt { "运行方式" } dd { match entry.runtime.as_deref() { Some("process") => "独立进程", Some("wasm-component") => "隔离组件", Some("page-definition") => "页面定义", _ => "插件声明的运行环境" } } dt { "网络访问" } dd { if entry.capabilities.network.is_empty() { "未申请" } else { "{entry.capabilities.network.join(\", \")}" } } dt { "文件访问" } dd { if entry.capabilities.filesystem.is_empty() { "未申请" } else { "{entry.capabilities.filesystem.join(\", \")}" } } dt { "数据库" } dd { if entry.capabilities.database { "已申请" } else { "未申请" } } } },
                "versions" => rsx! { if let Some(info)=information.as_ref() { super::releases::ReleaseHistory { info: info.clone(), entry: entry.clone(), busy, on_action } } else if let Some(Err(error))=details.read().as_ref() { az_ui_components::admin::RequestState { error: error.clone() } } else { az_ui_components::admin::RequestState {} } },
                _ => rsx! { if let Some(info)=information.as_ref() { if info.readme.is_empty() { p { "{entry.summary}" } p { class: "admin-meta", "此版本未提供 README。" } } else { super::documentation::ManagedMarkdown { source: info.readme.clone(), link_base, image_base } } } else if let Some(Err(error))=details.read().as_ref() { p { role: "alert", "{error}" } } else { p { role: "status", "正在读取 README" } } },
            }
        }
    }
}
