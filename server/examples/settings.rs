//! 浏览器验收专用的全屏设置页，使用正式 WIT 和真实 Wasm 执行器。

use az_plugin_guest::aio::plugin::metadata::{PageDefinition, Surface};
use az_plugin_guest::{Description, Guest, Phase, Request, Response};

struct Settings;

impl Guest for Settings {
    fn describe() -> Description {
        Description {
            label: "测试插件".into(),
            pages: vec![PageDefinition {
                id: "fixture".into(),
                label: "测试插件设置".into(),
                entry: "index.html".into(),
                scene: None,
                menu_path: vec![],
                permission: None,
                surface: Surface::Fullscreen,
            }],
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
            body: vec![],
        }
    }
}

az_plugin_guest::export!(Settings);
