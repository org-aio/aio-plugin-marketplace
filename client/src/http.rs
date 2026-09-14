use super::model::InstallRequest;
use gloo_net::http::{Request, Response};
use serde::Deserialize;

#[derive(Deserialize)]
struct RuntimeResponse<T> {
    data: T,
}

pub(super) async fn install(request: InstallRequest) -> Result<(), String> {
    let response = Request::post("/api/runtime/plugins/install")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    finish(response).await
}

pub(super) async fn action(source: &str, action: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("插件来源不存在".into());
    }
    let response = Request::post(&format!("/api/runtime/plugins/{source}/{action}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    finish(response).await
}

pub(super) async fn retry(id: &str) -> Result<(), String> {
    finish(
        Request::post(&format!("/api/runtime/delivery/jobs/{id}/retry"))
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await
}

async fn finish(response: Response) -> Result<(), String> {
    if !response.ok() {
        return Err(error(response).await);
    }
    dioxus::document::eval(
        "window.dispatchEvent(new Event('aio:catalog-invalidated')); return true;",
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(super) async fn get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let response = Request::get(path).send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(error(response).await);
    }
    response
        .json::<RuntimeResponse<T>>()
        .await
        .map(|r| r.data)
        .map_err(|e| e.to_string())
}

async fn error(response: Response) -> String {
    match response.status() {
        401 => return "会话已失效，请重新登录".into(),
        403 => return "你没有执行此操作的权限，请联系工作区管理员".into(),
        _ => {}
    }
    let body = response.text().await.unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_owned))
        .unwrap_or(body)
}
