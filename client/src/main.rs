use dioxus::prelude::*;

fn main() {
    // 导航由宿主 SDK 持有，避免 Dioxus 按构建 base_path 重写动态挂载地址和初始片段。
    let history = std::rc::Rc::new(dioxus::history::MemoryHistory::default());
    let config = dioxus::web::Config::default().history(history);
    dioxus::LaunchBuilder::web().with_cfg(config).launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        az_ui_components::UiStylesheets { relative_paths: true }
        aio_plugin_marketplace_client::MarketplacePage {}
    }
}
