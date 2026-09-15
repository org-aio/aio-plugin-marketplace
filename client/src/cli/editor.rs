use super::http;
use az_tool::{
    ToolManifest,
    registration::{Metadata, Registration},
};
use az_ui_components::{
    admin::{AsyncResult, EditorDialog},
    checkbox::{Checkbox, checkbox_is_checked, checkbox_state},
    input::{Input, TextInput},
    select::{Select, SelectItem},
};
use dioxus::prelude::*;

#[component]
pub(crate) fn RegisterDialog(
    on_close: Callback<()>,
    on_created: Callback<(ToolManifest, bool)>,
) -> Element {
    let mut command = use_signal(String::new);
    let mut git = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut uninstall = use_signal(String::new);
    let mut detect = use_signal(String::new);
    let mut platform = use_signal(|| "macos".to_owned());
    let mut install_now = use_signal(|| true);
    let mut created = use_signal(|| None::<ToolManifest>);
    use_future(move || async move {
        if let Ok(value) = document::eval("return /Windows/i.test(navigator.userAgent)?'windows':/Macintosh|Mac OS X/i.test(navigator.userAgent)?'macos':'linux';").await {
            if let Some(value) = value.as_str() { platform.set(value.into()); }
        }
    });
    rsx! {
        EditorDialog { title: "添加 CLI", description: "粘贴安装命令即可。Git 地址可选，填写后自动读取 README 作为插件说明。",
            submit_label: if install_now() { "保存并安装" } else { "保存到市场" }, pending_label: "正在保存并读取说明",
            save: move |_| -> AsyncResult<()> {
                let request = Registration { command: command(), metadata: Metadata { git: git(), title: title(), summary: summary() },
                    platforms: match platform().as_str() { "unix" => vec!["macos".into(), "linux".into()], "all" => vec!["macos".into(), "linux".into(), "windows".into()], value => vec![value.into()] },
                    uninstall: uninstall(), detect: detect() };
                Box::pin(async move { created.set(Some(http::register(&request).await?)); Ok(()) })
            },
            on_saved: move |_| {
                if let Some(manifest) = created() {
                    on_created.call((manifest, install_now()));
                }
            }, on_close,
            label { r#for: "cli-command", "安装命令" }
            Input { id: "cli-command", aria_label: "安装命令", required: true, maxlength: 4096, placeholder: "npx -y codex-model-sync@0.1.4 setup", value: command(), oninput: move |e: FormEvent| command.set(e.value()) }
            TextInput { label: "Git 仓库（可选）", value: git(), placeholder: "https://github.com/owner/repo", on_change: move |value| git.set(value) }
            label { r#for: "cli-platform", "命令适用系统" }
            Select { aria_label: "命令适用系统", value: platform(), on_value_change: move |value| platform.set(value),
                options: vec![("macos","macOS（Bash）"),("windows","Windows（PowerShell）"),("linux","Linux（Bash）"),("unix","macOS 和 Linux"),("all","三个系统均适用")].into_iter().map(|(value,label)| SelectItem::new(value,label)).collect(),
            }
            p { class: "admin-meta", "默认选择此电脑的系统；其他系统需要命令本身支持。" }
            label { class: "flex items-center gap-2", Checkbox { aria_label: "保存后打开本机助手安装", checked: Some(checkbox_state(install_now())), on_checked_change: move |value| install_now.set(checkbox_is_checked(value)) } "保存后打开本机助手安装" }
            details { summary { "标题、备注与更多选项" }
                TextInput { label: "标题（可选）", value: title(), placeholder: "自动使用仓库名或命令中的工具名", on_change: move |value| title.set(value) }
                TextInput { label: "备注（可选）", value: summary(), on_change: move |value| summary.set(value) }
                TextInput { label: "卸载命令（可选）", value: uninstall(), on_change: move |value| uninstall.set(value) }
                TextInput { label: "检测命令（可选）", value: detect(), on_change: move |value| detect.set(value) }
            }
            p { class: "admin-meta", "首次使用需执行 npx -y @zjarlin/aio helper install。安装将在本机终端确认后开始。" }
        }
    }
}

#[component]
pub(super) fn MetadataDialog(
    manifest: ToolManifest,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = manifest.id.clone();
    let mut title = use_signal(|| manifest.title.clone());
    let mut summary = use_signal(|| manifest.summary.clone());
    let mut git = use_signal(|| manifest.homepage.clone());
    rsx! {
        EditorDialog { title: "编辑 CLI 资料", description: "修改标题和备注。填写 Git 仓库后，保存时重新读取 README。",
            save: move |_| -> AsyncResult<()> { let id = id.clone(); let metadata = Metadata { title: title(), summary: summary(), git: git() };
                Box::pin(async move { http::update(&id, &metadata).await.map(|_| ()) }) }, on_close, on_saved,
            TextInput { label: "标题", value: title(), on_change: move |value| title.set(value) }
            TextInput { label: "备注", value: summary(), on_change: move |value| summary.set(value) }
            TextInput { label: "Git 仓库（可选）", value: git(), on_change: move |value| git.set(value) }
        }
    }
}
