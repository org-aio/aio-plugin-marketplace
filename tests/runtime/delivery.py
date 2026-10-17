"""真实产品、PostgreSQL 与市场整包验收；所有状态仅写入本轮随机测试数据库。"""

import gzip
import hashlib
import json
import os
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

plugin = Path(__file__).resolve().parents[2]
binary = Path(os.environ["AIO_MARKET_TEST_BINARY"]).resolve(strict=True)
packager = Path(os.environ["AIO_MARKET_TEST_PACKAGER"]).resolve(strict=True)
fixture_wasm = Path(os.environ["AIO_MARKET_TEST_FIXTURE_WASM"]).resolve(strict=True)
web = Path(os.environ["AIO_MARKET_TEST_WEB"]).resolve(strict=True)
database = urllib.parse.urlparse(os.environ["AIO_MARKET_TEST_DATABASE_URL"])
assert database.hostname == "127.0.0.1" and database.username == "postgres"
postgres = os.environ["AIO_MARKET_TEST_POSTGRES_CONTAINER"]
name = "marketplace_delivery_" + uuid.uuid4().hex[:12]
database = database._replace(path="/" + name).geturl()
root = Path(tempfile.mkdtemp(prefix="aio-marketplace-delivery-"))
(root / "bundles").mkdir()
source = root / "composition.json"
config = root / "aio.toml"
git = "https://github.com/org-aio/aio-plugin-marketplace.git"
fixture_git = "https://github.com/example/marketplace-fixture.git"
with socket.socket() as reservation:
    reservation.bind(("127.0.0.1", 0))
    port = reservation.getsockname()[1]
base = "http://127.0.0.1:%d" % port
env = {key: value for key, value in os.environ.items() if not key.startswith("AIO_")}
env.update({
    "AIO_DATABASE_URL": database, "AIO_COMPONENT_DATABASE_URL": database,
    "AIO_COMPONENT_HOME": str(root / "components"), "AIO_PLUGIN_CACHE": str(root / "cache"),
    "AIO_CONFIG": str(config), "AIO_FILE_STORAGE_DIR": str(root / "files"),
    "AIO_PROCESS_ROOT": str(root / "processes"),
    "AIO_PLUGIN_SUPERVISOR_SOCKET": str(root / "absent-supervisor.sock"),
    "AIO_RUNTIME_COMPOSITION": str(source), "AIO_PUBLIC_ORIGIN": base,
    "AIO_WEB_HOST": "127.0.0.1", "AIO_WEB_PORT": str(port), "AIO_WEB_DIST": str(web),
    "AIO_BOOTSTRAP_ACCOUNT": "market-test", "AIO_BOOTSTRAP_PASSWORD": uuid.uuid4().hex,
    "AIO_PLUGIN_PUBLISH_ACCOUNTS": "market-test",
})
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
cookie = None
host = None
log = open(str(root / "server.log"), "w", encoding="utf-8")
host_hash = hashlib.sha256(binary.read_bytes()).hexdigest()


def sql(query, db=name):
    return subprocess.check_output(["docker", "exec", postgres, "psql", "-U", "postgres",
        "-d", db, "-X", "-At", "-v", "ON_ERROR_STOP=1", "-c", query], universal_newlines=True).strip()


def request(path, value=None, raw=None, headers=None):
    headers = dict(headers or {})
    if cookie:
        headers["Cookie"] = cookie
    body = raw
    if value is not None:
        body = json.dumps(value).encode()
        headers["Content-Type"] = "application/json"
    try:
        response = http.open(urllib.request.Request(base + path, data=body, headers=headers), timeout=120)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def success(path, value=None, raw=None, headers=None):
    status, _, body = request(path, value, raw, headers)
    assert 200 <= status < 300, (path, status, body[:250])
    result = json.loads(body)
    return result.get("data", result)


def until(check):
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(0.1)
    raise AssertionError("等待超时，日志目录: " + str(root))


def ready():
    assert host.poll() is None, "宿主提前退出，日志目录: " + str(root)
    try:
        return request("/health")[0] == 200
    except OSError:
        return False


def launch():
    global host
    host = subprocess.Popen([str(binary)], cwd=str(root), env=env, stdout=log, stderr=subprocess.STDOUT)
    until(ready)
    assert request("/api/plugins/marketplace")[0] == 404


def stop():
    global host
    if host is not None and host.poll() is None:
        host.terminate()
        assert host.wait(timeout=40) == 0
    host = None


def package(directory, repository, version):
    output = root / ("bundles/" + ("market-" if repository == git else "fixture-") + version + ".aio-plugin")
    subprocess.check_call([str(packager), str(directory), "aio-plugin.toml", repository,
        "a" * 40, version, str(output)])
    return {"path": str(output), "revision": json.loads(gzip.decompress(output.read_bytes()))["digest"]}


def entry():
    return next(item for item in success("/api/runtime/marketplace") if item["git"] == git)


created = False
try:
    sql("CREATE DATABASE " + name, "postgres")
    created = True
    sql("REVOKE ALL ON SCHEMA public FROM PUBLIC")
    first = package(plugin, git, "0.1.0")
    second = package(plugin, git, "0.2.0")
    source.write_text(json.dumps({"schema_version": 1, "trusted_management_components": [
        {"git": git, "digest": bundle["revision"]} for bundle in [first, second]
    ]}), encoding="utf-8")
    config.write_text("plugins=[]\n[[bootstrap_components]]\npath='bundles/market-0.1.0.aio-plugin'\n"
        "digest='%s'\ntenants=['default']\n" % first["revision"], encoding="utf-8")
    launch()
    print("产品已启动，正在验证首次市场安装", flush=True)
    status, headers, body = request("/api/auth/login", {
        "account": env["AIO_BOOTSTRAP_ACCOUNT"], "password": env["AIO_BOOTSTRAP_PASSWORD"]})
    assert status == 200, (status, body[:150])
    cookie = headers["Set-Cookie"].split(";", 1)[0]
    assert entry()["active_revision"] == first["revision"]
    assert entry()["state"] == "active"
    first["source_id"] = entry()["source_id"]
    print("首次引导已验证，发布第二版和设置夹具", flush=True)
    publication = {"Content-Type": "application/vnd.aio.component+gzip"}
    success("/api/runtime/components/publish", raw=Path(second["path"]).read_bytes(), headers=publication)
    fixture = root / "fixture"
    (fixture / "web").mkdir(parents=True)
    shutil.copy2(str(fixture_wasm), str(fixture / "plugin.wasm"))
    (fixture / "web/index.html").write_text("<!doctype html><meta charset=\"utf-8\"><title>Fixture</title><p>测试插件设置</p>", encoding="utf-8")
    (fixture / "aio-plugin.toml").write_text("schema_version=2\n[plugin]\nsettings_page='fixture'\n"
        "[plugin.marketplace]\ntitle='生命周期测试插件'\nsummary='浏览器安装验收'\nlicense='MIT'\n"
        "[plugin.runtime]\nartifact='plugin.wasm'\nhost_version='>=2026.10.17'\n"
        "[plugin.frontend]\npath='web'\n", encoding="utf-8")
    fixture_versions = [package(fixture, fixture_git, version) for version in ["1.0.0", "2.0.0"]]
    for bundle in fixture_versions:
        published = success("/api/runtime/components/publish", raw=Path(bundle["path"]).read_bytes(), headers=publication)
        bundle["source_id"] = published["source_id"]
    # 通过正式文档存储提供中文 PNG 路径，浏览器必须经管理图片通道解码。
    revision = uuid.uuid4().hex + uuid.uuid4().hex
    document = uuid.uuid4().hex
    png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1cAAAAASUVORK5CYII="
    images = json.dumps({"图片.png": {"content_type": "image/png", "content_base64": png}})
    sql("INSERT INTO plugin_sources(id,git) VALUES('%s','https://example.com/%s.git');"
        "INSERT INTO plugin_revisions(id,source_id,revision,runtime,manifest,pages) VALUES('%s','%s','%s','page-definition','{}','[]');"
        "INSERT INTO plugin_packages(revision,git,version,manifest_toml,artifact_sha256,archive) VALUES('%s','https://example.com/%s.git','1.0.0','','%s','');"
        "INSERT INTO plugin_documents(revision,readme,images) VALUES('%s','','%s');" %
        (document, document, document, document, revision, revision, document, revision, revision, images))
    readme = "# 图片与说明\n\n![中文图片](/api/runtime/marketplace/%s/images/%%E5%%9B%%BE%%E7%%89%%87.png)\n\n" % revision
    readme += "\n\n".join("段落 %d：独立交付的插件说明。" % index for index in range(60))
    for bundle in fixture_versions:
        success("/api/runtime/components/%s/documentation" % bundle["revision"], raw=readme.encode(), headers={"Content-Type": "text/plain"})
    pairing = success("/api/runtime/workers/pairings", {"label": "测试设备", "platform": "linux",
        "capabilities": ["tools.install"], "machine_id": uuid.uuid4().hex})
    success("/api/runtime/workers/pairings/" + pairing["code"], {})
    device_headers = {"Authorization": "Bearer " + pairing["token"]}
    success("/api/runtime/workers/heartbeat", {}, headers=device_headers)
    success("/api/runtime/workers/tools/inventory", {"tools": {}, "packages": {}, "can_install": True}, headers=device_headers)
    session = root / "session.json"
    session.write_text(json.dumps({"base": base, "cookie": cookie, "first": first, "second": second,
        "fixture": fixture_versions, "fixture_git": fixture_git, "git": git, "device": pairing,
        "output": str(root / "browser")}), encoding="utf-8")
    session.chmod(0o600)
    print("进入浏览器验收，报告目录: " + str(root / "browser"), flush=True)
    subprocess.check_call(["docker", "run", "--rm", "--network", "host",
        "-v", str(plugin) + ":/work:ro", "-v", str(root) + ":" + str(root),
        "-v", os.environ["AIO_MARKET_TEST_NODE_MODULES"] + ":/node_modules:ro",
        "-e", "NODE_PATH=/node_modules", "-w", "/work", "mcr.microsoft.com/playwright:v1.62.0-noble",
        "node", "tests/browser/marketplace.cjs", str(session)])
    # 重启不重放首次引导，也不能把用户已经停用的市场重新启用。
    assert entry()["state"] == "disabled"
    stop()
    launch()
    assert entry()["state"] == "disabled"
    success("/api/runtime/plugins/%s/enable" % first["source_id"], {})
    assert entry()["active_revision"] == first["revision"]
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == host_hash
    report_path = root / "browser/report.json"
    report = json.loads(report_path.read_text(encoding="utf-8"))
    report.update({"disabledAfterRestart": True, "recoveredRevision": first["revision"],
        "hostBinaryUnchanged": True})
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print("市场整包、浏览器及重启恢复通过；报告: " + str(report_path))
finally:
    stop()
    log.close()
    if created:
        sql("DROP DATABASE " + name + " WITH (FORCE)", "postgres")
