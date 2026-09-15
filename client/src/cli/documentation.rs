use az_tool::{
    ToolManifest,
    registration::{Documentation, Metadata},
};
use az_ui_components::{
    admin::StatusMessage,
    button::{Button, ButtonVariant},
    markdown::Markdown,
};
use dioxus::prelude::*;

#[component]
pub(super) fn Readme(manifest: ToolManifest, can_manage: bool, refresh: u64) -> Element {
    let id = manifest.id.clone();
    let git = manifest.homepage.clone();
    let mut retry = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let resource = use_resource(use_reactive((&id, &git, &refresh), move |(id, git, _)| {
        let _ = retry();
        async move {
            if git.is_empty() {
                return Ok(Documentation::default());
            }
            super::super::http::get::<Documentation>(&format!("/api/runtime/tools/{id}/details"))
                .await
        }
    }));
    rsx! {
        section { aria_label: "CLI 插件说明",
            h2 { "插件说明" }
            if manifest.homepage.is_empty() { p { "{manifest.summary}" } }
            else {
                match resource.read().as_ref() {
                    Some(Ok(doc)) => rsx! {
                        if let Some(error) = &doc.error { p { role: "status", "{error}" } }
                        if !doc.readme.is_empty() { Markdown { source: doc.readme.clone(), link_base: doc.link_base.clone(), image_base: doc.image_base.clone() } }
                        else if doc.error.is_none() { p { "暂未获取 README。" } }
                    },
                    Some(Err(_)) => rsx! { p { "README 尚未获取，可打开项目仓库查看。" } },
                    None => rsx! { p { role: "status", "正在读取 README" } },
                }
                if can_manage { Button { variant: ButtonVariant::Outline, disabled: busy(), onclick: move |_| {
                    let id = manifest.id.clone();
                    let metadata = Metadata { git: manifest.homepage.clone(), title: manifest.title.clone(), summary: manifest.summary.clone() };
                    busy.set(true); error.set(None);
                    spawn(async move { match super::http::update(&id, &metadata).await { Ok(_) => retry += 1, Err(message) => error.set(Some(message)) } busy.set(false); });
                }, "刷新 README" } }
                if let Some(message) = error() { StatusMessage { error: true, message } }
            }
        }
    }
}
