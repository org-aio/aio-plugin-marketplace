use super::{
    details::PluginDetails,
    http,
    model::{InstallRequest, MarketplaceEntry},
    tree::PluginTree,
};
use az_ui_components::{
    admin::{AsyncResult, DeleteRecordsDialog, EditorDialog, RequestState, StatusMessage},
    button::Button,
    extension_browser::ExtensionBrowser,
};
use dioxus::prelude::*;

/// 独立市场前端入口；通过当前管理挂载读取目录并执行用户选择的操作。
#[allow(non_snake_case)]
pub fn MarketplacePage() -> Element {
    let mut registering = use_signal(|| false);
    let mut pending_install = use_signal(|| None::<az_tool::ToolManifest>);
    let navigation = super::navigation::use_navigation();
    let access = use_resource(|| async {
        http::get::<bool>("/api/runtime/tools/access")
            .await
            .unwrap_or(false)
    });
    let can_manage = access.read().as_ref().copied().unwrap_or(false);
    let mut entries = use_signal(Vec::<MarketplaceEntry>::new);
    let mut removing = use_signal(|| None::<MarketplaceEntry>);
    let mut delisting = use_signal(|| None::<MarketplaceEntry>);
    let mut busy = use_signal(|| false);
    let mut status = use_signal(|| None::<(bool, String)>);
    let mut loaded = use_signal(|| false);
    let mut load_error = use_signal(|| None::<String>);
    let mut refresh = use_signal(|| 0_u64);
    let _resource = use_resource(move || {
        let _ = refresh();
        async move {
            match http::get::<Vec<MarketplaceEntry>>("/api/runtime/marketplace").await {
                Ok(mut values) => {
                    values
                        .sort_by(|a, b| b.installed.cmp(&a.installed).then(a.title.cmp(&b.title)));
                    if *entries.peek() != values {
                        entries.set(values);
                    }
                    loaded.set(true);
                    load_error.set(None);
                }
                Err(error) => load_error.set(Some(error)),
            }
        }
    });
    use_future(move || async move {
        loop {
            if document::eval("await new Promise(resolve=>setTimeout(resolve,15000));return true;")
                .await
                .is_err()
            {
                break;
            }
            refresh += 1;
        }
    });
    let action = use_callback(move |(entry, action): (MarketplaceEntry, String)| {
        if entry.cli.is_some() {
            status.set(Some((true, "CLI 请在详情中选择目标设备".into())));
            return;
        }
        if busy() {
            return;
        }
        if action == "uninstall" {
            removing.set(Some(entry));
            return;
        }
        if action == "remove" {
            if can_manage {
                delisting.set(Some(entry));
            }
            return;
        }
        busy.set(true);
        status.set(None);
        spawn(async move {
            let result = if action == "install" || action.starts_with("install:") {
                http::install(InstallRequest {
                    git: entry.git.clone(),
                    rev: Some(action.strip_prefix("install:").unwrap_or(&entry.rev).into()),
                })
                .await
            } else if let Some(id) = action.strip_prefix("retry:") {
                http::retry(id).await
            } else {
                http::action(entry.source_id.as_deref().unwrap_or_default(), &action).await
            };
            busy.set(false);
            match result {
                Ok(()) => {
                    status.set(Some((false, "操作已完成".into())));
                    refresh += 1;
                }
                Err(error) => status.set(Some((true, error))),
            }
        });
    });
    let current = entries()
        .into_iter()
        .find(|entry| (navigation.view)().selected.as_deref() == Some(&entry.git));
    rsx! {
        ExtensionBrowser { detail_open: (navigation.view)().detail_open, sidebar_scroll_id: "marketplace-list", detail_scroll_id: "marketplace-detail",
            sidebar: rsx! {
                if can_manage { div { class: "extension-browser__search", Button { onclick: move |_| registering.set(true), "添加 CLI" } } }
                PluginTree { entries: entries(), selected: (navigation.view)().selected, search: (navigation.view)().search, installed_only: (navigation.view)().installed_only, cli_only: (navigation.view)().cli_only, loaded: loaded(),
                    on_filter: move |value| navigation.change(|view| {view.installed_only=value; view.selected=None; view.detail_open=false;}),
                    on_kind: move |value| navigation.change(|view| {view.cli_only=value; view.selected=None; view.detail_open=false;}),
                    on_search: move |value: String| navigation.change(|view| {view.search=value.trim().into(); view.selected=None; view.detail_open=false;}),
                    on_select: move |git| navigation.change(|view| {view.selected=Some(git); view.detail_open=true; view.tab="details".into();})
                }
                if let Some(error) = load_error() { RequestState { error, on_retry: move |_| refresh += 1 } }
                else if !loaded() { RequestState {} }
            },
            if let Some(message) = (navigation.error)() { StatusMessage { error: true, message } }
            if let Some((error,message)) = status() { StatusMessage { error, message } }
            if let Some(entry) = current {
                if let Some(manifest) = entry.cli.clone() { super::cli::CliDetails { key: "{entry.git}", manifest, can_manage, refresh: refresh(), on_updated: move |_| refresh += 1, on_back: move |_| navigation.change(|view|view.detail_open=false) } }
                else { PluginDetails { key: "{entry.git}", entry, entries: entries(), busy: busy(), can_manage, refresh: refresh(), on_action: move |value| action.call(value), on_back: move |_| navigation.change(|view|view.detail_open=false) } } }
            else if !loaded() { RequestState {} }
            else if (navigation.view)().selected.is_some() { az_ui_components::admin::EmptyState { title: "此插件不存在或无权访问", detail: "链接中的插件当前不可用。可以从列表选择其他插件。" } }
            else { az_ui_components::admin::EmptyState { title: "从插件开始扩展工作台", detail: "官方发布的插件会自动上架，在左侧选择插件查看介绍和安装。", a { href: "https://github.com/zjarlin/aio-platform/blob/main/docs/plugin/README.md", target: "_blank", rel: "noopener noreferrer", "查看中文开发指南 ↗" } } }
        }
        if registering() { super::cli::RegisterDialog { on_close: move |_| registering.set(false), on_created: move |(manifest, install_now): (az_tool::ToolManifest, bool)| {
            if install_now { pending_install.set(Some(manifest.clone())); }
            registering.set(false);
            navigation.change(|view| { view.selected=Some(format!("aio-tool:{}",manifest.id)); view.detail_open=true; view.installed_only=false; view.search.clear(); });
            refresh += 1;
        } } }
        if let Some(manifest) = pending_install() {
            EditorDialog { title: "通过本机助手安装", description: format!("{} 已保存到市场。打开本机助手后，在终端确认安装步骤。",manifest.title), submit_label: "打开本机助手", pending_label: "正在打开",
                save: move |_| -> AsyncResult<()> { let parameters=serde_json::json!({"id":manifest.id,"version":manifest.version}); Box::pin(async move { http::interaction("install",parameters).await }) },
                on_close: move |_| pending_install.set(None), on_saved: move |_| pending_install.set(None),
                p { "首次使用请先执行 npx -y @zjarlin/aio helper install。" }
            }
        }
        if let Some(entry) = removing() {
            DeleteRecordsDialog { title: "卸载插件", confirm_label: "确认卸载", warning: "移除当前租户的插件页面与运行实例，保留业务数据和版本历史。", items: vec![entry], item_label: |entry: MarketplaceEntry| entry.title,
                delete: |entry: MarketplaceEntry| -> AsyncResult<()> { Box::pin(async move { http::action(entry.source_id.as_deref().unwrap_or_default(),"uninstall").await }) },
                on_close: move |_| removing.set(None), on_deleted: move |_| { removing.set(None); refresh += 1; },
            }
        }
        if let Some(entry) = delisting() {
            DeleteRecordsDialog { title: "从市场删除", confirm_label: "确认删除",
                warning: "对所有用户下架此插件，禁止新的安装。已安装实例、业务数据和版本历史保留；后续发布不会自动重新上架。",
                items: vec![entry], item_label: |entry: MarketplaceEntry| entry.title,
                delete: |entry: MarketplaceEntry| -> AsyncResult<()> { Box::pin(async move { http::remove(&entry.git).await }) },
                on_close: move |_| delisting.set(None),
                on_deleted: move |_| { delisting.set(None); navigation.change(|view|view.detail_open=false); refresh += 1; },
            }
        }
    }
}
