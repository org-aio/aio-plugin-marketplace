use az_tool::{InstallLink, ToolManifest};
use az_ui_components::{
    button::{Button, ButtonVariant},
    markdown::Markdown,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowLeft, Terminal};

#[component]
pub(crate) fn CliDetails(
    manifest: ToolManifest,
    on_back: Callback<()>,
    can_manage: bool,
    on_updated: Callback<()>,
    refresh: u64,
) -> Element {
    let mut editing = use_signal(|| false);
    let link = InstallLink {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
    }
    .to_string();
    let mut platform = use_signal(|| "macos".to_owned());
    use_future(move || async move {
        if let Ok(value) = document::eval("const p=navigator.userAgent;return /Windows/i.test(p)?'windows':/Macintosh|Mac OS X/i.test(p)?'macos':'linux';").await {
            if let Some(value) = value.as_str() { platform.set(value.into()); }
        }
    });
    let plan = manifest.platforms.get(&platform()).cloned();
    let install = format!(
        "npx -y @zjarlin/aio tool install {} --version {}",
        manifest.id, manifest.version
    );
    let uninstall = format!("npx -y @zjarlin/aio tool uninstall {}", manifest.id);
    rsx! {
        Button { class: "extension-browser__back", variant: ButtonVariant::Ghost, onclick: move |_| on_back.call(()), ArrowLeft {} "插件列表" }
        header { class: "extension-browser__heading", Terminal {} div {
            h1 { "{manifest.title}" }
            p { "CLI" }
            p { "{manifest.summary}" }
        } }
        div { class: "extension-browser__actions",
            if plan.is_some() { a { class: "dx-button", "data-style": "primary", "data-size": "default", href: link, "安装到本机" } }
        }
        if can_manage { Button { variant: ButtonVariant::Outline, onclick: move |_| editing.set(true), "编辑标题和备注" } }
        super::documentation::Readme { manifest: manifest.clone(), can_manage, refresh }
        p { class: "admin-meta", "安装会在本机终端显示命令并等待确认。完成后可运行 npx -y @zjarlin/aio tool list 查看结果。" }
        nav { class: "extension-browser__tabs", role: "tablist", aria_label: "CLI 安装平台",
            for (id,label) in [("macos","macOS"),("windows","Windows"),("linux","Linux")] {
                button { role: "tab", "aria-selected": platform()==id, onclick: move |_| platform.set(id.into()), "{label}" }
            }
        }
        div { role: "tabpanel",
            if let Some(plan) = plan {
                { let removal = if plan.uninstall.is_empty() { "此条目未提供卸载命令，请按照项目说明手动卸载。".to_owned() } else { format!("卸载：\n\n```sh\n{uninstall}\n```") }; rsx! { Markdown { link_base: manifest.homepage.clone(), image_base: manifest.homepage.clone(), source: format!("## 首次连接此电脑\n\n在终端执行一次，注册本机助手；之后可直接点击上方安装。需要 Node.js 和 npm。\n\n```sh\nnpx -y @zjarlin/aio helper install\n```\n\n## 安装与卸载\n\n也可以在终端执行：\n\n```sh\n{install}\n```\n\n{removal}") } } }
                details { summary { "查看依赖和实际操作" }
                    for requirement in &plan.requirements { p { "{requirement.label}：{requirement.help}" } }
                    h3 { "安装步骤" }
                    for command in &plan.install { pre { code { "{command.program} {serde_json::to_string(&command.args).unwrap_or_default()}" } } }
                    h3 { "卸载步骤" }
                    if plan.uninstall.is_empty() { p { "此条目未提供卸载命令，请按照项目说明手动卸载。" } }
                    for command in &plan.uninstall { pre { code { "{command.program} {serde_json::to_string(&command.args).unwrap_or_default()}" } } }
                }
            } else { p { role: "status", "此版本暂未提供该平台的安装步骤。" } }
            if !manifest.homepage.is_empty() { a { href: manifest.homepage.clone(), target: "_blank", rel: "noopener noreferrer", "项目主页 ↗" } }
        }
        if editing() { super::editor::MetadataDialog { manifest: manifest.clone(), on_close: move |_| editing.set(false), on_saved: move |_| { editing.set(false); on_updated.call(()); } } }
    }
}
