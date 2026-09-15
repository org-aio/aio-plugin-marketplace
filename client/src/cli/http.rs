use az_tool::{
    ToolManifest,
    registration::{Documentation, Metadata, Registration},
};
use gloo_net::http::Request;
use serde::Deserialize;

#[derive(Deserialize)]
struct Response<T> {
    data: T,
}

pub(super) async fn register(request: &Registration) -> Result<ToolManifest, String> {
    send(Request::post("/api/runtime/tools/register"), request).await
}

pub(super) async fn update(id: &str, metadata: &Metadata) -> Result<Documentation, String> {
    send(
        Request::patch(&format!("/api/runtime/tools/{id}/details")),
        metadata,
    )
    .await
}

async fn send<T: for<'de> Deserialize<'de>>(
    request: gloo_net::http::RequestBuilder,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let response = request
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(super::super::http::error(response).await);
    }
    response
        .json::<Response<T>>()
        .await
        .map(|r| r.data)
        .map_err(|e| e.to_string())
}
