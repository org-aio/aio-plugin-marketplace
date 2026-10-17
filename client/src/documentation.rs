//! 文档继续使用共享 Markdown，只把图片通过当前授权挂载读取并管理 Blob 生命周期。

use az_ui_components::{
    admin::StatusMessage,
    markdown::{Markdown, image_sources},
};
use dioxus::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

const PLACEHOLDER: &str =
    "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

#[derive(Deserialize)]
struct ImageEvent {
    source: String,
    blob: Option<String>,
    error: Option<String>,
}

#[component]
pub(super) fn ManagedMarkdown(source: String, link_base: String, image_base: String) -> Element {
    let mut images = use_signal(BTreeMap::<String, String>::new);
    let mut errors = use_signal(Vec::<String>::new);
    let mut active = use_signal(|| None::<document::Eval>);
    let mut generation = use_signal(|| 0_u64);
    use_effect(use_reactive(
        (&source, &image_base),
        move |(source, base)| {
            if let Some(previous) = *active.peek() {
                let _ = previous.send(());
            }
            active.set(None);
            let epoch = *generation.peek() + 1;
            generation.set(epoch);
            images.set(BTreeMap::new());
            errors.set(Vec::new());
            let mut sources = image_sources(&source, &base);
            if sources.len() > 64 {
                sources.truncate(64);
                errors.set(vec!["文档图片超过 64 张，仅加载前 64 张。".into()]);
            }
            if sources.is_empty() {
                return;
            }
            let mut bridge = document::eval(include_str!("documentation.js"));
            if let Err(error) = bridge.send(sources) {
                errors.write().push(format!("文档图片读取失败：{error}"));
                return;
            }
            active.set(Some(bridge));
            spawn(async move {
                loop {
                    let result = bridge.recv::<ImageEvent>().await;
                    if *generation.peek() != epoch {
                        break;
                    }
                    match result {
                        Ok(event) => {
                            if let Some(blob) = event.blob {
                                images.write().insert(event.source, blob);
                            }
                            if let Some(error) = event.error {
                                errors.write().push(error);
                            }
                        }
                        Err(error) => {
                            errors.write().push(format!("文档图片通道已关闭：{error}"));
                            break;
                        }
                    }
                }
            });
        },
    ));
    use_drop(move || {
        if let Some(bridge) = *active.peek() {
            let _ = bridge.send(());
        }
    });
    rsx! {
        Markdown { source, link_base, image_base,
            image_url: move |source: String| images.read().get(&source).cloned().unwrap_or_else(||PLACEHOLDER.into()),
        }
        if !errors.read().is_empty() {
            StatusMessage { error: true, message: format!("部分文档图片未能加载：{}", errors.read().join("；")) }
        }
    }
}
