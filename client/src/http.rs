use super::model::InstallRequest;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Deserialize)]
struct RuntimeResponse<T> {
    data: T,
}

pub(super) async fn install(request: InstallRequest) -> Result<(), String> {
    send::<serde_json::Value, _>("POST", "/api/runtime/plugins/install", Some(&request))
        .await
        .map(|_| ())
}

pub(super) async fn remove(git: &str) -> Result<(), String> {
    send::<serde_json::Value, _>(
        "DELETE",
        "/api/runtime/marketplace",
        Some(&serde_json::json!({"git":git})),
    )
    .await
    .map(|_| ())
}

pub(super) async fn action(source: &str, action: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("插件来源不存在".into());
    }
    send::<serde_json::Value, ()>(
        "POST",
        &format!("/api/runtime/plugins/{source}/{action}"),
        None,
    )
    .await
    .map(|_| ())
}

/// 设置或更新一条导航标签覆盖；只发送新的显示名。
pub(super) async fn set_navigation_label(kind: &str, key: &str, label: &str) -> Result<(), String> {
    send::<serde_json::Value, _>(
        "POST",
        "/api/runtime/navigation/labels",
        Some(&serde_json::json!({"kind":kind,"key":key,"label":label})),
    )
    .await
    .map(|_| ())
}

/// 恢复整包声明的默认显示名。
pub(super) async fn clear_navigation_label(kind: &str, key: &str) -> Result<(), String> {
    send::<serde_json::Value, _>(
        "DELETE",
        "/api/runtime/navigation/labels",
        Some(&serde_json::json!({"kind":kind,"key":key})),
    )
    .await
    .map(|_| ())
}

pub(super) async fn retry(id: &str) -> Result<(), String> {
    send::<serde_json::Value, ()>(
        "POST",
        &format!("/api/runtime/delivery/jobs/{id}/retry"),
        None,
    )
    .await
    .map(|_| ())
}

pub(super) async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    send::<T, ()>("GET", path, None).await
}

// 序列化数据跨消息边界传递，JSON 管理结果、错误与空响应在此统一解码。
pub(super) async fn send<T: DeserializeOwned, B: Serialize + ?Sized>(
    method: &str,
    path: &str,
    body: Option<&B>,
) -> Result<T, String> {
    let mut bridge = dioxus::document::eval(
        r#"
        const input = await dioxus.recv();
        try {
            const response = await window.aioPlugin.manage({ method: input.method, path: input.path,
                headers: input.body === null ? [] : [{name:'content-type',value:'application/json'}],
                body: input.body === null ? new Uint8Array() : new TextEncoder().encode(JSON.stringify(input.body)) });
            const text = new TextDecoder().decode(response.body);
            let value;
            try { value = text ? JSON.parse(text) : {data:null}; } catch (_) { value = {error:text}; }
            if (response.status < 200 || response.status >= 300) throw new Error(value.error || `HTTP ${response.status}`);
            dioxus.send({value});
        } catch (error) { dioxus.send({error:String(error.message ?? error)}); }
    "#,
    );
    bridge
        .send(serde_json::json!({"method":method,"path":path,"body":body}))
        .map_err(|error| error.to_string())?;
    let response: serde_json::Value = bridge.recv().await.map_err(|error| error.to_string())?;
    if let Some(error) = response["error"].as_str() {
        return Err(error.into());
    }
    serde_json::from_value::<RuntimeResponse<T>>(response["value"].clone())
        .map(|response| response.data)
        .map_err(|error| error.to_string())
}

// 三个宿主动作只传递语义参数，真实地址和手势授权由正式 SDK 处理。
pub(super) async fn interaction(
    operation: &str,
    parameters: serde_json::Value,
) -> Result<(), String> {
    let mut bridge = dioxus::document::eval(
        r#"
        const input = await dioxus.recv();
        try {
            const {aioPlugin:sdk} = window;
            switch (input.operation) {
                case 'download': await sdk.downloadPackage(input.parameters.revision); break;
                case 'settings': await sdk.openSettings(input.parameters.page_id); break;
                case 'install': await sdk.installTool(input.parameters.id, input.parameters.version); break;
                default: throw new Error('未知宿主操作');
            }
            dioxus.send({});
        } catch (error) { dioxus.send({error:String(error.message ?? error)}); }
    "#,
    );
    bridge
        .send(serde_json::json!({"operation":operation,"parameters":parameters}))
        .map_err(|error| error.to_string())?;
    let response: serde_json::Value = bridge.recv().await.map_err(|error| error.to_string())?;
    match response["error"].as_str() {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}
