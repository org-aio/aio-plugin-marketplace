use super::{
    details::PluginDetails,
    http,
    model::{InstallRequest, MarketplaceEntry},
    tree::PluginTree,
};
use az_ui_components::{
    admin::{AsyncResult, DeleteRecordsDialog, RequestState, StatusMessage},
    extension_browser::ExtensionBrowser,
};
use dioxus::prelude::*;

#[allow(non_snake_case)]
pub(super) fn MarketplacePage() -> Element {
    let mut entries = use_signal(Vec::<MarketplaceEntry>::new);
    let mut selected = use_signal(|| None::<String>);
    let mut search = use_signal(String::new);
    let mut installed_only = use_signal(|| false);
    let mut detail_open = use_signal(|| false);
    let mut removing = use_signal(|| None::<MarketplaceEntry>);
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
                    if selected()
                        .as_ref()
                        .is_none_or(|git| !values.iter().any(|entry| &entry.git == git))
                    {
                        selected.set(values.first().map(|entry| entry.git.clone()));
                    }
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
        if let Ok(value) = document::eval("return {selected:sessionStorage.getItem('aio-marketplace-selected'),search:sessionStorage.getItem('aio-marketplace-search'),installed:sessionStorage.getItem('aio-marketplace-installed')};").await {
            if let Some(value) = value.get("selected").and_then(|v| v.as_str()) { selected.set(Some(value.into())); }
            if let Some(value) = value.get("search").and_then(|v| v.as_str()) { search.set(value.into()); }
            installed_only.set(value.get("installed").and_then(|v| v.as_str()) == Some("true"));
        }
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
    use_effect(move || {
        let value = serde_json::json!({"selected":selected(),"search":search(),"installed":installed_only()});
        spawn(async move {
            let _ = document::eval(&format!("const value={value};try{{if(value.selected)sessionStorage.setItem('aio-marketplace-selected',value.selected);sessionStorage.setItem('aio-marketplace-search',value.search);sessionStorage.setItem('aio-marketplace-installed',value.installed);}}catch(_){{}}return true;")).await;
        });
    });
    let action = use_callback(move |(entry, action): (MarketplaceEntry, String)| {
        if busy() {
            return;
        }
        if action == "uninstall" {
            removing.set(Some(entry));
            return;
        }
        busy.set(true);
        status.set(None);
        spawn(async move {
            let result = if action == "install" {
                http::install(InstallRequest {
                    git: entry.git.clone(),
                    rev: Some(entry.rev.clone()),
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
        .find(|entry| selected().as_deref() == Some(&entry.git));
    rsx! {
        ExtensionBrowser { detail_open: detail_open(),
            sidebar: rsx! {
                PluginTree { entries: entries(), selected: selected(), search: search(), installed_only: installed_only(), loaded: loaded(), on_filter: move |value| installed_only.set(value), on_search: move |value| search.set(value), on_select: move |git| { selected.set(Some(git)); detail_open.set(true); } }
                if let Some(error) = load_error() { RequestState { error, on_retry: move |_| refresh += 1 } }
                else if !loaded() { RequestState {} }
            },
            if let Some((error,message)) = status() { StatusMessage { error, message } }
            if let Some(entry) = current { PluginDetails { key: "{entry.git}", entry, entries: entries(), busy: busy(), refresh: refresh(), on_action: move |value| action.call(value), on_back: move |_| detail_open.set(false) } }
            else if !loaded() { RequestState {} }
            else { az_ui_components::admin::EmptyState { title: "从插件开始扩展工作台", detail: "官方发布的插件会自动上架，在左侧选择插件查看介绍和安装。", a { href: "https://github.com/zjarlin/aio-platform/blob/main/docs/plugin/README.md", target: "_blank", rel: "noopener noreferrer", "查看中文开发指南 ↗" } } }
        }
        if let Some(entry) = removing() {
            DeleteRecordsDialog { title: "卸载插件", confirm_label: "确认卸载", warning: "移除当前租户的插件页面与运行实例，保留业务数据和版本历史。", items: vec![entry], item_label: |entry: MarketplaceEntry| entry.title,
                delete: |entry: MarketplaceEntry| -> AsyncResult<()> { Box::pin(async move { http::action(entry.source_id.as_deref().unwrap_or_default(),"uninstall").await }) },
                on_close: move |_| removing.set(None), on_deleted: move |_| { removing.set(None); refresh += 1; },
            }
        }
    }
}
