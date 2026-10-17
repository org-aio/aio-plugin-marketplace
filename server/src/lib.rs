//! 市场组件只声明页面和生命周期，所有管理动作复用宿主的授权契约。

use az_plugin_guest::aio::plugin::metadata::{PageDefinition, Scene, Surface};
use az_plugin_guest::{Description, Guest, Phase, Request, Response};

struct Marketplace;

impl Guest for Marketplace {
    fn describe() -> Description {
        Description {
            label: "插件市场".into(),
            pages: vec![
                PageDefinition {
                    id: "marketplace".into(),
                    label: "插件市场".into(),
                    entry: "index.html".into(),
                    scene: Some(Scene {
                        id: "system".into(),
                        label: "系统".into(),
                    }),
                    menu_path: vec![],
                    permission: None,
                    surface: Surface::Workspace,
                },
                PageDefinition {
                    id: "marketplace-account".into(),
                    label: "插件市场".into(),
                    entry: "index.html".into(),
                    scene: None,
                    menu_path: vec![],
                    permission: None,
                    surface: Surface::AccountEntry,
                },
            ],
        }
    }

    fn health() -> Result<(), String> {
        Ok(())
    }
    fn lifecycle(_: Phase) -> Result<(), String> {
        Ok(())
    }
    fn handle(_: Request) -> Response {
        Response {
            status: 404,
            headers: vec![],
            body: "此组件通过宿主管理契约提供服务".as_bytes().to_vec(),
        }
    }
}

az_plugin_guest::export!(Marketplace);
