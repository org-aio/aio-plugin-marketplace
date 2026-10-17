# 市场整包验收

先执行 `scripts/build.sh` 构建市场，使用当前 aio-platform 构建 `az-plugin-bundle` 的 `package` 示例；执行 `cargo build -p aio-plugin-marketplace-server --example settings --release --target wasm32-wasip2` 构建全屏设置夹具。然后在 aio-idea 编译 server，并运行 `dx serve --platform web --release --debug-symbols false` 生成实际产品 Web。产品保留 Dioxus 默认加载脚本；仅市场打包脚本使用自有相对路径启动入口。浏览器使用安装了 Playwright 的 Node.js 依赖目录和 `mcr.microsoft.com/playwright:v1.62.0-noble`。

设置以下环境变量后运行 `python3 tests/runtime/delivery.py`：

- `AIO_MARKET_TEST_BINARY`：当前 aio-idea server 可执行文件。
- `AIO_MARKET_TEST_PACKAGER`：正式 `az-plugin-bundle` 的 `package` 示例程序。
- `AIO_MARKET_TEST_FIXTURE_WASM`：上一步构建的 `examples/settings.wasm`。
- `AIO_MARKET_TEST_WEB`：本轮 `dx serve --release` 的产品 `release/web/public` 目录；由正式产品服务托管，不包含开发 WebSocket 连接。
- `AIO_MARKET_TEST_DATABASE_URL`：本机专用测试 PostgreSQL 的连接 URL，用户必须为 postgres。
- `AIO_MARKET_TEST_POSTGRES_CONTAINER`：该测试 PostgreSQL 容器名称。
- `AIO_MARKET_TEST_NODE_MODULES`：已安装 Playwright 的 node_modules 绝对路径。

脚本创建并最终删除随机 `marketplace_delivery_*` 数据库，保留临时目录中的日志、桌面和手机截图及报告。登录密码、会话和设备令牌仅用于本轮测试，不提交入库，不把临时 session.json 分享为报告。

测试覆盖固定摘要首次引导、工作区和账户入口、图片、原始整包下载、插件设置、安装指定版本、回滚、自升级、自停用、重启保留停用意图，以及搜索、页签、分享 URL、新浏览器上下文、刷新、前进后退与滚动。CLI 使用正式注册和元数据管理接口，点击本机助手时只验证已授权协议地址。配对设备通过真实配对、任务领取、结果和清单回报接口模拟；不执行真实电脑上的软件安装命令。

运行时管理通道的无信任、跨租户、撤权、过期挂载、服务停用、恶意图片地址和超过 32 MiB 下载边界，另由 aio-platform 的 `component_executors`、`components/management_tests.rs` 与 `tests/browser/component-lifecycle.cjs` 验证。
