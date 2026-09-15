mod cli;
mod details;
mod family;
mod http;
mod model;
mod page;
mod releases;
mod tree;

use az_dioxus_admin_shell::{ApplicationPage, ApplicationPlugin, ApplicationScene};
use dill::CatalogBuilder;

#[derive(Debug)]
pub struct MarketplacePlugin;
impl ApplicationPlugin for MarketplacePlugin {
    fn pages(&self) -> Vec<ApplicationPage> {
        vec![ApplicationPage {
            id: "marketplace",
            label: "插件市场",
            icon: Some("store"),
            scene: ApplicationScene {
                id: "system",
                label: "系统",
            },
            menu_path: Vec::new(),
            required_permission: Some("plugin:manage"),
            render: page::MarketplacePage,
        }]
    }
}
pub fn register(builder: &mut CatalogBuilder) {
    builder
        .add_value(MarketplacePlugin)
        .bind::<dyn ApplicationPlugin, MarketplacePlugin>();
}
