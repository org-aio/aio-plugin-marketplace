use super::{
    family::{Node, forest},
    model::MarketplaceEntry,
};
use az_ui_components::{
    admin::EmptyState,
    button::{Button, ButtonSize, ButtonVariant},
    input::Input,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronDown, ChevronRight, Package};

#[component]
pub(super) fn PluginTree(
    entries: Vec<MarketplaceEntry>,
    selected: Option<String>,
    search: String,
    installed_only: bool,
    on_filter: Callback<bool>,
    loaded: bool,
    on_search: Callback<String>,
    on_select: Callback<String>,
) -> Element {
    let mut installed_open = use_signal(|| true);
    let mut available_open = use_signal(|| true);
    let no_matches = forest(&entries, true, search.trim()).is_empty()
        && (installed_only || forest(&entries, false, search.trim()).is_empty());
    rsx! {
        h1 { class: "extension-browser__title", "插件市场" }
        div { class:"extension-browser__search", Input {aria_label:"搜索插件",placeholder:"搜索插件",value:search.clone(),oninput:move|e:FormEvent|on_search.call(e.value())} }
        div { class: "extension-browser__filters", role: "group", aria_label: "插件筛选",
            for (only, label) in [(false, "全部"), (true, "已安装")] { Button { size: ButtonSize::Sm, variant: ButtonVariant::Ghost, aria_pressed: (installed_only == only).to_string(), onclick: move |_| on_filter.call(only), "{label}" } }
        }
        if loaded && no_matches {
            if entries.is_empty() { EmptyState { title: "还没有发布的插件", detail: "插件构建和发布成功后会自动出现在这里。" } }
            else if search.trim().is_empty() { EmptyState { title: "当前工作区尚未安装插件" } }
            else { EmptyState { title: "没有找到匹配的插件", Button { variant: ButtonVariant::Ghost, onclick: move |_| on_search.call(String::new()), "清空搜索" } } }
        }
        div {role:"tree",aria_label:"插件",onkeydown:move|e:KeyboardEvent| {
            let key=match e.key(){Key::ArrowDown=>"down",Key::ArrowUp=>"up",Key::Home=>"home",Key::End=>"end",_=>return};
            e.prevent_default();
            spawn(async move {let _=document::eval(&format!("const items=[...document.querySelectorAll('[role=treeitem]')].filter(e=>e.getClientRects().length);const i=items.indexOf(document.activeElement);const key='{key}';const n=key==='home'?0:key==='end'?items.length-1:key==='up'?Math.max(0,i-1):Math.min(items.length-1,i+1);items[n]?.focus();items[n]?.click();return true;")).await;});
        },
            for (installed,label) in [(true,"已安装"),(false,"可安装")].into_iter().filter(|(installed,_)| !installed_only || *installed) {
                details {class:"extension-browser__group",open:if installed{installed_open()}else{available_open()},
                    summary {onclick:move|e|{e.prevent_default();if installed{installed_open.toggle();}else{available_open.toggle();}},"{label} ({entries.iter().filter(|e|e.installed==installed).count()})"}
                    div {role:"group",for node in forest(&entries,installed,&search) {
                        Branch {key:"{node.git}",node,level:1,selected:selected.clone(),on_select}
                    }}
                }
            }
        }
    }
}

#[component]
fn Branch(
    node: Node,
    level: usize,
    selected: Option<String>,
    on_select: Callback<String>,
) -> Element {
    let mut open = use_signal(|| true);
    let has_children = !node.children.is_empty();
    let title = node.title;
    rsx! {
        div {class:"extension-browser__branch",
            div {class:"extension-browser__row",
                if has_children {button {class:"extension-browser__toggle",aria_label:format!("{} {title}",if open(){"收起"}else{"展开"}),title:if open(){"收起子插件"}else{"展开子插件"},onclick:move|_|open.toggle(),if open(){ChevronDown{}}else{ChevronRight{}}}}
                if let Some(entry)=node.entry {
                    button {class:"extension-browser__item",role:"treeitem","aria-level":level,"aria-expanded":has_children.then(||open().to_string()),"aria-selected":selected.as_deref()==Some(&entry.git),tabindex:0,
                        onclick:{let git=entry.git.clone();move|_|on_select.call(git.clone())},
                        onkeydown:move|e:KeyboardEvent|{if e.key()==Key::ArrowLeft && has_children{e.prevent_default();open.set(false);}else if e.key()==Key::ArrowRight && has_children{e.prevent_default();open.set(true);}},
                        Package{} span {strong{"{entry.title}"}p{"{entry.summary}"}small{"{entry.state_label()}"}}
                    }
                }else {div {class:"extension-browser__item",Package{}span{strong{"{title}"}small{"父插件尚未发布"}}}}
            }
            if has_children && open() {div{class:"extension-browser__children",role:"group",for child in node.children {Branch{key:"{child.git}",node:child,level:level+1,selected:selected.clone(),on_select}}}}
        }
    }
}
