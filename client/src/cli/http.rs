use super::super::http::send;
use az_tool::{
    ToolManifest,
    registration::{Documentation, Metadata, Registration},
};

pub(super) async fn register(request: &Registration) -> Result<ToolManifest, String> {
    send("POST", "/api/runtime/tools/register", Some(request)).await
}

pub(super) async fn update(id: &str, metadata: &Metadata) -> Result<Documentation, String> {
    send(
        "PATCH",
        &format!("/api/runtime/tools/{id}/details"),
        Some(metadata),
    )
    .await
}

pub(super) async fn remove(id: &str) -> Result<(), String> {
    send("DELETE", &format!("/api/runtime/tools/{id}"), None::<&()>).await
}

pub(super) async fn install(id: &str, body: &serde_json::Value) -> Result<(), String> {
    send::<serde_json::Value, _>(
        "POST",
        &format!("/api/runtime/tools/{id}/install"),
        Some(body),
    )
    .await
    .map(|_| ())
}
