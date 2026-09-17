use az_tool::{InstallLink, ToolManifest};
use az_ui_components::{
    admin::{AsyncResult, EditorDialog, RequestState, StatusMessage},
    button::{Button, ButtonVariant},
};
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize)]
struct Installation {
    version: String,
    state: String,
}

#[derive(Clone, PartialEq, Deserialize)]
struct Device {
    id: String,
    label: String,
    platform: String,
    status: String,
    installed: Option<Installation>,
    checked_at: Option<i64>,
    fresh: bool,
    can_install: bool,
    supported: bool,
    error: Option<String>,
}

#[derive(Clone, Deserialize)]
struct Task {
    worker_id: String,
    capability: String,
    state: String,
    input: serde_json::Value,
    error: Option<String>,
}

impl Device {
    fn state_label(&self) -> String {
        let state = match &self.installed {
            Some(record) => format!(
                "{} {}",
                match record.state.as_str() {
                    "installed" => "已安装",
                    "executed" => "安装命令已完成",
                    "failed" => "安装失败",
                    "uninstalling" => "卸载未完成",
                    "missing" => "软件未检测到，保留安装记录",
                    _ => "安装未完成",
                },
                record.version
            ),
            None if self.checked_at.is_none() => "待检测 · 请升级并启动设备助手".into(),
            None if self.error.is_some() => "检测失败".into(),
            None if !self.fresh => "安装状态待刷新".into(),
            None => "未检测到安装".into(),
        };
        if !self.fresh && self.installed.is_some() {
            format!("{state}（上次记录）")
        } else {
            state
        }
    }
}

#[component]
pub(super) fn DeviceInstallations(
    manifest: ToolManifest,
    refresh: u64,
    on_updated: Callback<()>,
) -> Element {
    let id = manifest.id.clone();
    let mut pending = use_signal(|| None::<Device>);
    let mut revision = use_signal(|| 0_u64);
    let resource = use_resource(use_reactive((&id, &refresh), move |(id, _)| {
        let _ = revision();
        async move {
            let devices =
                super::super::http::get::<Vec<Device>>(&format!("/api/runtime/tools/{id}/devices"))
                    .await?;
            let tasks = super::super::http::get::<Vec<Task>>("/api/runtime/workers/tasks").await?;
            Ok::<_, String>((
                devices,
                tasks
                    .into_iter()
                    .filter(|t| {
                        t.capability == "tools.install" && t.input["id"].as_str() == Some(&id)
                    })
                    .collect::<Vec<_>>(),
            ))
        }
    }));
    let state = resource.read().clone();
    rsx! {
        section { class: "admin-section", aria_label: "设备安装状态",
            h2 { "设备安装状态" }
            p { class: "admin-meta", "选择要安装的设备。已安装版本来自设备回报；本机指浏览器所在电脑。" }
            match state {
                None => rsx! { RequestState {} },
                Some(Err(error)) => rsx! { RequestState { error, on_retry: move |_| revision += 1 } },
                Some(Ok((devices, tasks))) => rsx! {
                    if devices.is_empty() {
                        p { role: "status", "尚未配对设备，无法确认本机安装状态。" }
                        a { class: "dx-button", "data-style": "outline", href: InstallLink { id: manifest.id.clone(), version: manifest.version.clone() }.to_string(), "通过本机助手安装" }
                    }
                    for device in devices {
                        {
                            let task = tasks.iter().find(|t| t.worker_id == device.id);
                            let busy = task.is_some_and(|t| ["queued", "running"].contains(&t.state.as_str()));
                            let label = device.state_label();
                            let can_install = device.fresh && device.can_install && device.supported && device.installed.is_none() && !busy;
                            rsx! {
                                article { key: "{device.id}", class: "flex flex-wrap items-center justify-between gap-2 border-b pb-3",
                                    div { class: "grid gap-2",
                                    strong { "{device.label}" }
                                    small { class: "admin-meta", {match device.platform.as_str() { "darwin" | "macos" => "macOS", "win32" | "windows" => "Windows", _ => "Linux" }} " · " if device.status == "online" { "在线" } else { "离线" } }
                                    span { class: "admin-status", "data-enabled": device.installed.as_ref().is_some_and(|r| r.state == "installed"), role: "status", "{label}" }
                                    if let Some(error) = device.error.clone() { StatusMessage { error: true, message: error } }
                                    if busy { p { role: "status", "正在等待设备完成安装…" } }
                                    if let Some(task) = task {
                                        if let Some(error) = task.error.clone() { StatusMessage { error: true, message: error } }
                                        else if task.state == "complete" { p { role: "status", "设备已完成安装任务" } }
                                    }
                                    if !device.supported { p { class: "admin-meta", "此版本不支持该设备系统" } }
                                    else if device.installed.is_none() && !device.can_install && device.status == "online" {
                                        p { class: "admin-meta", "升级设备上的 AIO 与 Space 助手后，可在此安装。" }
                                    }
                                    }
                                    if can_install { Button { onclick: { let device = device.clone(); move |_| pending.set(Some(device.clone())) }, "安装到此设备" } }
                                }
                            }
                        }
                    }
                    div { class: "admin-actions", Button { variant: ButtonVariant::Outline, onclick: move |_| { revision += 1; on_updated.call(()); }, "刷新设备状态" } }
                },
            }
        }
        if let Some(device) = pending() {
            EditorDialog { title: "确认安装到设备", description: format!("将 {} {} 安装到 {}，由该设备的助手执行安装步骤。", manifest.title, manifest.version, device.label), submit_label: "确认安装", pending_label: "正在提交",
                save: { let id = manifest.id.clone(); let version = manifest.version.clone(); move |_| -> AsyncResult<()> {
                    let body = serde_json::json!({"worker_id": device.id, "version": version});
                    let id = id.clone();
                    Box::pin(async move { super::http::install(&id, &body).await })
                } },
                on_close: move |_| pending.set(None), on_saved: move |_| { pending.set(None); revision += 1; on_updated.call(()); },
                p { "安装完成后，以设备回报的结果为准。" }
            }
        }
    }
}
