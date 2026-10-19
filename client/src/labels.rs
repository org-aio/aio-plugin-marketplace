use super::{
    http,
    model::{CatalogPage, NavigationCatalog, NavigationLabel},
};
use az_ui_components::{
    admin::StatusMessage,
    button::{Button, ButtonSize, ButtonVariant},
    input::TextInput,
};
use dioxus::prelude::*;

/// 重命名目标：场景页签、菜单分组、页面入口或账户入口。
#[derive(Clone, Debug, PartialEq)]
struct Target {
    kind: &'static str,
    key: String,
    /// 整包声明的原始显示名，用于恢复默认与回显。
    default_label: String,
    /// 覆盖生效后的显示名；未覆盖时等于默认值。
    label: String,
    location: String,
}

impl Target {
    fn overridden(&self) -> bool {
        self.label != self.default_label
    }
}

/// 收集租户目录里的可重命名元素；只列出当前已授权的页面与账户入口。
fn targets(catalog: &NavigationCatalog, source: &str, labels: &[NavigationLabel]) -> Vec<Target> {
    let overlay = |kind: &str, key: &str, default_label: &str| {
        labels
            .iter()
            .find(|entry| entry.kind == kind && entry.key == key)
            .map(|entry| entry.label.clone())
            .unwrap_or_else(|| default_label.to_owned())
    };
    let mut targets = Vec::new();
    let mut scenes: Vec<(String, String)> = Vec::new();
    for page in catalog.pages.iter().filter(|page| owns(page, source)) {
        let Some(scene) = page.scene.as_ref() else {
            continue;
        };
        if !scenes.iter().any(|(id, _)| id == &scene.id) {
            scenes.push((scene.id.clone(), scene.label.clone()));
        }
    }
    for (id, label) in scenes {
        targets.push(Target {
            kind: "scene",
            label: overlay("scene", &id, &label),
            default_label: label,
            key: id,
            location: "场景页签".into(),
        });
    }
    for page in catalog.pages.iter().filter(|page| owns(page, source)) {
        for group in &page.menu_path {
            if targets
                .iter()
                .any(|target| target.kind == "menu" && target.key == group.id)
            {
                continue;
            }
            targets.push(Target {
                kind: "menu",
                label: overlay("menu", &group.id, &group.label),
                default_label: group.label.clone(),
                key: group.id.clone(),
                location: format!("菜单分组 · {}", page.label),
            });
        }
        targets.push(Target {
            kind: "page",
            label: overlay("page", &page.id, &page.label),
            default_label: page.label.clone(),
            key: page.id.clone(),
            location: "工作区入口".into(),
        });
    }
    for item in catalog.account_items.iter() {
        let Some(page_id) = item.page_id.as_deref() else {
            continue;
        };
        if !owns_page(catalog, page_id, source) {
            continue;
        }
        targets.push(Target {
            kind: "page",
            label: overlay("page", &item.id, &item.label),
            default_label: item.label.clone(),
            key: item.id.clone(),
            location: "账户入口".into(),
        });
    }
    targets
}

fn owns(page: &CatalogPage, source: &str) -> bool {
    owns_page_id(&page.id, source)
}

fn owns_page(catalog: &NavigationCatalog, page_id: &str, source: &str) -> bool {
    catalog
        .pages
        .iter()
        .any(|page| page.id == page_id && owns(page, source))
}

/// 页面标识形如 `component:<source>:<page>`；只有动态整包页面可被租户覆盖。
fn owns_page_id(page_id: &str, source: &str) -> bool {
    page_id
        .strip_prefix("component:")
        .is_some_and(|rest| rest.starts_with(source) && rest[source.len()..].starts_with(':'))
}

/// 单个插件的导航重命名面板；键、类别与默认值都来自宿主目录。
#[component]
pub(super) fn NavigationLabels(
    git: String,
    title: String,
    busy: bool,
    refresh: u64,
    on_close: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let git_key = git.clone();
    let catalog = use_resource(use_reactive!(|git_key, refresh| async move {
        let _ = refresh;
        let git = git_key;
        let catalog: NavigationCatalog = http::get("/api/runtime/catalog").await?;
        // 目录里的 plugins 用安装来源标识与 Git 地址区分已安装整包。
        let source = catalog
            .plugins
            .iter()
            .find(|plugin| plugin.git == git)
            .map(|plugin| plugin.source_id.clone());
        Ok::<_, String>((catalog, source))
    }));
    let mut status = use_signal(|| None::<(bool, String)>);
    let mut pending = use_signal(|| false);
    let mut edits = use_signal(std::collections::BTreeMap::<String, String>::new);

    let loaded = catalog.read();
    // 目录读取结果在进入 rsx 之前收敛成固定状态，避免在宏内做匹配与遮蔽。
    let (targets_ready, load_error, source) = match loaded.as_ref() {
        Some(Ok((catalog, source))) => (
            targets(
                catalog,
                source.as_deref().unwrap_or_default(),
                &catalog.navigation_labels,
            ),
            None,
            source.clone().unwrap_or_default(),
        ),
        Some(Err(error)) => (Vec::new(), Some(error.clone()), String::new()),
        None => {
            return rsx! {
                section { class: "extension-browser__optional", aria_label: "导航名称",
                    p { role: "status", "正在读取目录" }
                }
            };
        }
    };
    rsx! {
        section { class: "extension-browser__optional", aria_label: "导航名称",
            h2 { "导航名称" }
            p { class: "admin-meta", "只改写“{title}”在页签、菜单和账户入口中的显示名；页面标识、权限与挂载地址保持不变。其他租户的设置互相独立。" }
            if let Some(message) = status() { StatusMessage { error: message.0, message: message.1 } }
            if let Some(error) = load_error { StatusMessage { error: true, message: error } }
            if targets_ready.is_empty() {
                p { class: "admin-meta", "此插件当前没有可重命名的导航入口。" }
            }
            for target in targets_ready {
                { target_row(target, &mut edits, &mut status, &mut pending, busy, on_saved) }
            }
            div { class: "extension-browser__child-action",
                span { class: "admin-meta", "来源 {source}" }
                Button { size: ButtonSize::Sm, variant: ButtonVariant::Ghost, onclick: move |_| on_close.call(()), "关闭" }
            }
        }
    }
}

/// 单个可重命名元素：回显当前名称，并提供保存与恢复默认。
fn target_row(
    target: Target,
    edits: &mut Signal<std::collections::BTreeMap<String, String>>,
    status: &mut Signal<Option<(bool, String)>>,
    pending: &mut Signal<bool>,
    busy: bool,
    on_saved: EventHandler<()>,
) -> Element {
    let id = format!("{}:{}", target.kind, target.key);
    let current = edits
        .read()
        .get(&id)
        .cloned()
        .unwrap_or_else(|| target.label.clone());
    let default_label = target.default_label.clone();
    let location = target.location.clone();
    let key = target.key.clone();
    let kind = target.kind;
    let overridden = target.overridden();
    let edit_id = id.clone();
    let save_key = key.clone();
    let save_value = current.clone();
    let restore_key = key.clone();
    let restore_id = id.clone();
    // Signal 是 Copy 句柄；按值捕获副本，闭包才能实现 FnMut。
    let mut edit_signal = *edits;
    let mut status_signal = *status;
    let mut pending_signal = *pending;
    rsx! {
        div { class: "extension-browser__child-action", key: "{id}",
            div {
                strong { "{location}" }
                p { class: "admin-meta", "默认：{default_label}" }
            }
            TextInput {
                label: format!("{location}显示名"),
                value: current.clone(),
                on_change: move |value| { edit_signal.write().insert(edit_id.clone(), value); },
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Outline,
                disabled: busy || pending() || current.trim().is_empty() || current == default_label,
                onclick: move |_| {
                    if save_value.len() > 128 {
                        status_signal.set(Some((true, "名称不能超过 128 个字符".into())));
                        return;
                    }
                    pending_signal.set(true);
                    status_signal.set(None);
                    let key = save_key.clone();
                    let value = save_value.clone();
                    spawn(async move {
                        let result = http::set_navigation_label(kind, &key, value.trim()).await;
                        pending_signal.set(false);
                        match result {
                            Ok(()) => { status_signal.set(Some((false, "已更新显示名".into()))); on_saved.call(()); }
                            Err(error) => status_signal.set(Some((true, error))),
                        }
                    });
                },
                "保存名称"
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Ghost,
                disabled: busy || pending() || !overridden,
                onclick: move |_| {
                    pending_signal.set(true);
                    status_signal.set(None);
                    let key = restore_key.clone();
                    let id = restore_id.clone();
                    spawn(async move {
                        let result = http::clear_navigation_label(kind, &key).await;
                        pending_signal.set(false);
                        match result {
                            Ok(()) => { edit_signal.write().remove(&id); status_signal.set(Some((false, "已恢复默认名称".into()))); on_saved.call(()); }
                            Err(error) => status_signal.set(Some((true, error))),
                        }
                    });
                },
                "恢复默认"
            }
        }
    }
}
