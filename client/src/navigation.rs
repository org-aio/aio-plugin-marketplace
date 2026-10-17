//! 已提交的选择、筛选和业务页签由外层 URL 驱动；表单和搜索草稿保留在页面内。

use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct View {
    pub selected: Option<String>,
    pub search: String,
    pub installed_only: bool,
    pub cli_only: bool,
    pub detail_open: bool,
    pub tab: String,
    pub platform: Option<String>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            selected: None,
            search: String::new(),
            installed_only: false,
            cli_only: false,
            detail_open: false,
            tab: "details".into(),
            platform: None,
        }
    }
}

impl View {
    fn parse(fragment: &str) -> Self {
        let mut value = Self::default();
        let mut seen = std::collections::HashSet::new();
        for (key, input) in
            url::form_urlencoded::parse(fragment.strip_prefix("#?").unwrap_or_default().as_bytes())
        {
            if !seen.insert(key.clone()) {
                continue;
            }
            match key.as_ref() {
                "source" if !input.is_empty() && input.len() <= 1024 => {
                    value.selected = Some(input.into_owned())
                }
                "q" if input.chars().count() <= 128 => value.search = input.trim().to_owned(),
                "installed" => value.installed_only = input == "1",
                "kind" => value.cli_only = input == "cli",
                "detail" => value.detail_open = input == "1",
                "tab" if ["details", "versions", "permissions"].contains(&input.as_ref()) => {
                    value.tab = input.into_owned()
                }
                "platform" if ["macos", "windows", "linux"].contains(&input.as_ref()) => {
                    value.platform = Some(input.into_owned())
                }
                _ => {}
            }
        }
        value
    }

    fn fragment(&self) -> String {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        if let Some(source) = &self.selected {
            query.append_pair("source", source);
        }
        if !self.search.is_empty() {
            query.append_pair("q", &self.search);
        }
        if self.installed_only {
            query.append_pair("installed", "1");
        }
        if self.cli_only {
            query.append_pair("kind", "cli");
        }
        if self.detail_open {
            query.append_pair("detail", "1");
        }
        if self.tab != "details" {
            query.append_pair("tab", &self.tab);
        }
        if let Some(platform) = &self.platform {
            query.append_pair("platform", platform);
        }
        let query = query.finish();
        if query.is_empty() {
            String::new()
        } else {
            format!("#?{query}")
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Navigation {
    pub view: ReadSignal<View>,
    pub error: ReadSignal<Option<String>>,
    navigate: Callback<View>,
}

impl Navigation {
    pub fn change(self, update: impl FnOnce(&mut View)) {
        let mut target = self.view.read().clone();
        update(&mut target);
        if target != *self.view.peek() {
            self.navigate.call(target);
        }
    }
}

pub(super) fn use_navigation() -> Navigation {
    let mut view = use_signal(|| {
        let fragment = web_sys::window()
            .and_then(|window| window.location().hash().ok())
            .unwrap_or_default();
        View::parse(&fragment)
    });
    let mut error = use_signal(|| None::<String>);
    let mut listener = use_signal(|| None::<document::Eval>);
    use_future(move || async move {
        let mut events = document::eval(
            r#"
            const dispose = window.aioPlugin.onNavigationChange(fragment => dioxus.send(fragment));
            await dioxus.recv(); dispose();
        "#,
        );
        listener.set(Some(events));
        while let Ok(fragment) = events.recv::<String>().await {
            let next = View::parse(&fragment);
            if *view.peek() != next {
                view.set(next);
            }
        }
    });
    use_drop(move || {
        if let Some(listener) = *listener.peek() {
            let _ = listener.send(());
        }
    });
    let navigate = use_callback(move |next: View| {
        error.set(None);
        spawn(async move {
            let bridge = document::eval(
                r#"
                const fragment = await dioxus.recv();
                try { await window.aioPlugin.navigate(fragment); return null; }
                catch (error) { return String(error.message ?? error); }
            "#,
            );
            if let Err(cause) = bridge.send(next.fragment()) {
                error.set(Some(cause.to_string()));
                return;
            }
            match bridge.await {
                Ok(value) => {
                    if let Some(message) = value.as_str() {
                        error.set(Some(message.into()));
                    }
                }
                Err(cause) => error.set(Some(cause.to_string())),
            }
        });
    });
    let navigation = Navigation {
        view: view.into(),
        error: error.into(),
        navigate,
    };
    use_context_provider(|| navigation);
    navigation
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_roundtrip_unicode_selection_filters_and_tabs() {
        let view = View {
            selected: Some("https://github.com/example/tool.git".into()),
            search: "中文 & CLI".into(),
            installed_only: true,
            cli_only: true,
            detail_open: true,
            tab: "versions".into(),
            platform: Some("linux".into()),
        };
        assert_eq!(View::parse(&view.fragment()), view);
        assert_eq!(View::parse(&View::parse(&view.fragment()).fragment()), view);
        assert_eq!(View::default().fragment(), "");
    }
    #[test]
    fn invalid_optional_parameters_and_duplicate_keys_have_stable_defaults() {
        let view = View::parse("#?tab=bad&platform=bad&installed=yes&q=first&q=second");
        assert_eq!(view.tab, "details");
        assert_eq!(view.platform, None);
        assert!(!view.installed_only);
        assert_eq!(view.search, "first");
        assert_eq!(View::parse(&view.fragment()), view);
    }
}
