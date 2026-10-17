# AIO 插件市场

独立 v2 Wasm Component 整包，同时贡献工作区页面和账户入口。产品不链接市场的 Rust 前后端；安装、升级、回滚、停用和卸载由运行时处理。

## 构建与首次引导

使用仓库固定的 Rust nightly、Dioxus CLI 0.7.9 与 Node.js，执行 `scripts/build.sh`。产物为 `dist/marketplace.wasm` 和 `dist/frontend`，通过 AIO 正式打包命令生成 `.aio-plugin`；源码推送不等同于包已发布或已安装。

运行时至少为 `2026.10.17`，部署方需要审核整包，并把规范 SHA256 摘要加入 `AIO_RUNTIME_COMPOSITION`：

```json
{
  "schema_version": 1,
  "trusted_management_components": [
    {"git": "https://github.com/org-aio/aio-plugin-marketplace.git", "digest": "<已审核整包的 64 位摘要>"}
  ]
}
```

在 `AIO_CONFIG` 指向的配置中声明首次引导，路径相对于该配置文件：

```toml
[[bootstrap_components]]
path = "bundles/marketplace.aio-plugin"
digest = "<同一个整包摘要>"
tenants = ["default"]
```

部署必须配置组件数据库与持久存储。首次引导只补充未安装记录；不覆盖租户已选版本、停用或卸载意图。新版需要单独审核并加入信任，之后可在市场中升级自身；旧版保留信任时可回滚。移除信任会暂停对应管理能力。市场停用后的恢复使用正式运行时安装接口或部署恢复流程，不由包自行提升权限。

## 实现入口

先看 `server/src/lib.rs` 的 WIT 页面声明，再看 `client/src/page.rs` 的操作流程。`client/src/http.rs` 通过注入的 `aioPlugin.manage` 调用正式宿主路由；大包下载、插件设置与本机助手安装使用有真实手势校验的语义动作。配对设备安装保留确认、排队、设备回报和错误状态。

`client/src/navigation.rs` 把选择、搜索、筛选、页签及 CLI 平台写入外层 URL。`client/src/documentation.rs` 复用共享 Markdown，图片经 `managementImage` 获取并在文档切换或卸载后释放 Blob。包不能直接使用宿主 Cookie、绕过已停用服务或代理任意站内请求。

## 验证

`cargo test -p aio-plugin-marketplace-client --lib` 检查目录及 URL 契约；`cargo clippy -p aio-plugin-marketplace-client --target wasm32-unknown-unknown -- -D warnings` 检查实际 Web 目标。完整验收使用 `dx serve --platform web` 构建产品页面、真实 PostgreSQL、打包后的市场及浏览器；步骤见 `tests/README.md`。
