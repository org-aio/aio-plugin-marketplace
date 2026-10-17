const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { chromium } = require('playwright');

async function main() {
  const test = JSON.parse(await fs.readFile(process.argv[2], 'utf8'));
  assert.equal(new URL(test.base).hostname, '127.0.0.1');
  await fs.mkdir(test.output, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const errors = [];
  const failedResponses = [];
  const pageId = `component:${test.first.source_id}:marketplace`;
  const marketUrl = `${test.base}/?page=${encodeURIComponent(pageId)}`;
  async function context(mobile = false) {
    const ctx = await browser.newContext({ viewport: mobile ? { width: 390, height: 844 } : { width: 1440, height: 900 } });
    const split = test.cookie.indexOf('=');
    await ctx.addCookies([{ name: test.cookie.slice(0, split), value: test.cookie.slice(split + 1), url: test.base, httpOnly: true }]);
    ctx.on('page', page => {
      page.setDefaultTimeout(60000);
      page.on('pageerror', error => errors.push(error.message));
      page.on('console', message => {
        if (message.type() === 'error' && !message.text().includes("Failed to launch 'aio://")) errors.push(message.text());
      });
      page.on('response', response => {
        if (response.status() < 400) return;
        failedResponses.push((async () => ({
          method: response.request().method(), status: response.status(),
          endpoint: new URL(response.url()).pathname.split('/').at(-1),
          error: (await response.text().catch(() => '')).slice(0, 500),
        }))());
      });
    });
    return ctx;
  }
  const ctx = await context();
  const page = await ctx.newPage();
  const frame = page.frameLocator('iframe[title="插件市场"]:visible');
  async function json(method, pathname, data, headers) {
    const response = await ctx.request.fetch(test.base + pathname, { method, data, headers });
    assert(response.ok(), `${pathname}: HTTP ${response.status()} ${(await response.text()).slice(0, 200)}`);
    const value = await response.json();
    return value.data ?? value;
  }
  async function entry(repository = test.git) {
    return (await json('GET', '/api/runtime/marketplace')).find(item => item.git === repository);
  }
  async function until(check) {
    const deadline = Date.now() + 60000;
    while (Date.now() < deadline) {
      if (await check()) return;
      await new Promise(resolve => setTimeout(resolve, 150));
    }
    throw new Error('等待权威状态超时');
  }
  async function open(url = marketUrl, target = page) {
    await target.goto(url);
    await target.locator('.application-shell:visible').waitFor();
    await target.frameLocator('iframe[title="插件市场"]:visible').locator('.extension-browser').waitFor();
  }
  async function select(title, target = frame) {
    await target.getByRole('treeitem').filter({ hasText: title }).click();
    await target.getByRole('heading', { name: title, exact: true }).last().waitFor();
  }
  async function menu(label) {
    if (!await frame.getByRole('menuitem', { name: label, exact: true }).isVisible()) {
      await frame.getByLabel('管理插件', { exact: true }).click();
    }
    await frame.getByRole('menuitem', { name: label, exact: true }).click();
  }
  try {
    await open();
    console.log('已挂载市场整包');
    await select('生命周期测试插件');
    await frame.getByRole('img', { name: '中文图片' }).waitFor();
    await until(() => frame.getByRole('img', { name: '中文图片' }).evaluate(img => img.complete && img.naturalWidth === 1 && img.src.startsWith('blob:')));
    await frame.getByRole('button', { name: '安装', exact: true }).click();
    await until(async () => (await entry(test.fixture_git)).active_revision === test.fixture[1].revision);
    await frame.getByRole('button', { name: '插件设置', exact: true }).click();
    await page.getByRole('dialog').waitFor();
    await page.getByRole('dialog').frameLocator('iframe').getByText('测试插件设置', { exact: true }).waitFor();
    await page.getByRole('dialog').getByRole('button', { name: /关闭/ }).click();
    console.log('已验证图片、安装和设置');
    const downloadPromise = page.waitForEvent('download');
    await menu('下载插件包');
    const download = await downloadPromise;
    assert.deepEqual(await fs.readFile(await download.path()), await fs.readFile(test.fixture[1].path));
    await frame.getByRole('tab', { name: '版本与发布', exact: true }).click();
    await until(() => Promise.resolve(new URL(page.url()).searchParams.get('route')?.includes('tab=versions')));
    await frame.getByRole('button', { name: '安装此版本', exact: true }).first().click();
    await until(async () => (await entry(test.fixture_git)).active_revision === test.fixture[0].revision);
    await frame.getByRole('button', { name: '更新到最新版本', exact: true }).click();
    await until(async () => (await entry(test.fixture_git)).active_revision === test.fixture[1].revision);
    await menu('回退版本');
    await until(async () => (await entry(test.fixture_git)).active_revision === test.fixture[0].revision);
    console.log('已验证下载、指定版本、升级和回滚');
    await frame.getByRole('tab', { name: '权限', exact: true }).click();
    await until(() => Promise.resolve(new URL(page.url()).searchParams.get('route')?.includes('tab=permissions')));
    await page.goBack();
    await until(() => frame.getByRole('tab', { name: '版本与发布', exact: true }).getAttribute('aria-selected').then(value => value === 'true'));
    await page.goForward();
    await until(() => frame.getByRole('tab', { name: '权限', exact: true }).getAttribute('aria-selected').then(value => value === 'true'));
    await frame.getByRole('tab', { name: '介绍', exact: true }).click();
    await frame.getByRole('img', { name: '中文图片' }).waitFor();
    await page.screenshot({ path: path.join(test.output, 'desktop.png') });
    await frame.locator('[data-url-scroll="marketplace-detail"]').hover();
    await page.mouse.wheel(0, 700);
    await until(() => frame.locator('[data-url-scroll="marketplace-detail"]').evaluate(element => element.scrollTop >= 650));
    await until(() => Promise.resolve(new URL(page.url()).searchParams.has('scroll')));
    const shared = page.url();
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert(await frame.locator('html').evaluate(element => element.scrollWidth <= innerWidth));
    await page.screenshot({ path: path.join(test.output, 'desktop-scroll.png') });
    const freshContext = await context();
    const fresh = await freshContext.newPage();
    await open(shared, fresh);
    const freshFrame = fresh.frameLocator('iframe[title="插件市场"]:visible');
    await freshFrame.getByRole('heading', { name: '生命周期测试插件', exact: true }).waitFor();
    await until(() => freshFrame.locator('[data-url-scroll="marketplace-detail"]').evaluate(element => element.scrollTop >= 650));
    await fresh.reload();
    await freshFrame.getByRole('heading', { name: '生命周期测试插件', exact: true }).waitFor();
    const missing = new URL(marketUrl);
    missing.searchParams.set('route', '#?source=https%3A%2F%2Fexample.com%2Fmissing.git&detail=1&tab=bad&tab=permissions');
    await open(missing.href, fresh);
    await freshFrame.getByText('此插件不存在或无权访问', { exact: true }).waitFor();
    await fresh.close();
    console.log('已验证下载、版本、回滚和分享链接');
    const mobileContext = await context(true);
    const mobile = await mobileContext.newPage();
    await open(shared, mobile);
    const mobileFrame = mobile.frameLocator('iframe[title="插件市场"]:visible');
    await mobileFrame.getByRole('heading', { name: '生命周期测试插件', exact: true }).waitFor();
    await until(() => mobileFrame.locator('[data-url-scroll="marketplace-detail"]').evaluate(element => element.scrollTop >= 650));
    await mobileFrame.locator('[data-url-scroll="marketplace-detail"]').hover();
    await mobile.mouse.wheel(0, -1000);
    await until(() => mobileFrame.locator('[data-url-scroll="marketplace-detail"]').evaluate(element => element.scrollTop === 0));
    assert(await mobile.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert(await mobileFrame.locator('html').evaluate(element => element.scrollWidth <= innerWidth));
    await mobile.screenshot({ path: path.join(test.output, 'mobile.png') });
    await mobileFrame.getByRole('button', { name: '插件列表', exact: true }).click();
    await mobileFrame.getByRole('textbox', { name: '搜索插件', exact: true }).fill('不存在的插件');
    await mobileFrame.getByRole('button', { name: '搜索', exact: true }).click();
    await mobileFrame.getByText('没有找到匹配的插件', { exact: true }).waitFor();
    assert(mobile.url().includes('route='));
    await mobile.goBack();
    await mobileFrame.getByRole('treeitem').first().waitFor();
    await mobile.close();
    console.log('已验证手机视口');
    // 工作区和账户都由真实安装目录贡献，账户入口不依赖产品静态注册。
    await open();
    await page.locator('.application-shell__sidebar button[aria-label$="的账户菜单"]').click();
    await page.getByRole('menuitem', { name: '插件市场', exact: true }).click();
    await frame.getByRole('treeitem').first().waitFor();
    assert(new URL(page.url()).searchParams.get('account')?.includes(test.first.source_id));
    await open();
    // 模拟助手进入安装流程前的心跳和清单回报，设备在线状态不依赖前面浏览器步骤的耗时。
    const deviceHeaders = { Authorization: `Bearer ${test.device.token}` };
    await json('POST', '/api/runtime/workers/heartbeat', {}, deviceHeaders);
    await json('POST', '/api/runtime/workers/tools/inventory', { tools: {}, packages: {}, can_install: true }, deviceHeaders);
    await frame.getByRole('button', { name: '添加 CLI', exact: true }).click();
    await frame.getByRole('textbox', { name: '安装命令', exact: true }).fill('printf marketplace-test');
    await frame.getByRole('checkbox', { name: '保存后打开本机助手安装', exact: true }).uncheck();
    await frame.getByRole('button', { name: '保存到市场', exact: true }).click();
    await frame.getByRole('heading', { name: '设备安装状态', exact: true }).waitFor();
    const tool = (await json('GET', '/api/runtime/marketplace')).find(item => item.cli?.id.startsWith('cli-'));
    assert(tool);
    await frame.getByRole('tab', { name: 'Linux', exact: true }).click();
    await until(() => Promise.resolve(new URL(page.url()).searchParams.get('route')?.includes('platform=linux')));
    await frame.getByRole('button', { name: '编辑标题和备注', exact: true }).click();
    await frame.getByRole('textbox', { name: '标题', exact: true }).fill('验收 CLI');
    await frame.getByRole('textbox', { name: '备注', exact: true }).fill('配对设备管理验收');
    await frame.getByRole('button', { name: '保存', exact: true }).click();
    await frame.getByRole('heading', { name: '验收 CLI', exact: true }).waitFor();
    // 本机动作只检查经宿主批准的协议地址，测试环境不会执行系统安装。
    const devtools = await ctx.newCDPSession(page);
    await devtools.send('Page.enable');
    const external = new Promise(resolve => devtools.on('Page.frameRequestedNavigation', event => {
      if (event.url.startsWith('aio://')) resolve(event.url);
    }));
    await frame.getByRole('button', { name: '通过本机助手安装', exact: true }).click();
    assert.equal(await Promise.race([external, new Promise((_, reject) => setTimeout(() => reject(new Error('本机助手动作未发生')), 15000))]), `aio://install/${tool.cli.id}?version=${tool.cli.version}`);
    await devtools.detach();
    console.log('已验证 CLI 注册、元数据和本机助手动作');
    await frame.getByRole('button', { name: '安装到此设备', exact: true }).click();
    await frame.getByRole('dialog').getByRole('button', { name: '确认安装', exact: true }).click();
    await frame.getByText('正在等待设备完成安装…', { exact: true }).waitFor();
    const task = await json('POST', '/api/runtime/workers/claim', { request_id: randomUUID(), wait_seconds: 0 }, deviceHeaders);
    assert.equal(task.capability, 'tools.install');
    assert.equal(task.input.id, tool.cli.id);
    await json('POST', `/api/runtime/workers/tasks/${task.id}/complete`, { lease: task.lease, result: {}, error: null }, deviceHeaders);
    await json('POST', '/api/runtime/workers/tools/inventory', { tools: { [tool.cli.id]: { version: tool.cli.version, state: 'installed' } }, packages: {}, can_install: true }, deviceHeaders);
    await frame.getByRole('button', { name: '刷新设备状态', exact: true }).click();
    await frame.getByText(`已安装 ${tool.cli.version}`, { exact: true }).waitFor();
    await frame.getByText('设备已完成安装任务', { exact: true }).waitFor();
    await page.screenshot({ path: path.join(test.output, 'devices.png') });
    console.log('已验证配对设备任务和回报');
    await frame.getByRole('button', { name: '从插件市场移除', exact: true }).click();
    await frame.getByRole('dialog').getByRole('button', { name: '取消', exact: true }).click();
    assert(await entry(tool.git));
    await open();
    await select('插件市场');
    const originalFrame = await page.locator('iframe[title="插件市场"]:visible').getAttribute('id');
    await frame.getByRole('button', { name: '更新到最新版本', exact: true }).click();
    await until(async () => (await entry()).active_revision === test.second.revision);
    await until(async () => {
      const id = await page.locator('iframe[title="插件市场"]:visible').getAttribute('id');
      return id && id !== originalFrame;
    });
    await frame.getByRole('treeitem').first().waitFor();
    await select('插件市场');
    const upgradedFrame = await page.locator('iframe[title="插件市场"]:visible').getAttribute('id');
    await menu('回退版本');
    await until(async () => (await entry()).active_revision === test.first.revision);
    await until(async () => {
      const id = await page.locator('iframe[title="插件市场"]:visible').getAttribute('id');
      return id && id !== upgradedFrame;
    });
    await frame.getByRole('treeitem').first().waitFor();
    await select('插件市场');
    await menu('停用');
    await until(async () => (await entry()).state === 'disabled');
    await until(() => page.locator('iframe[title="插件市场"]:visible').count().then(count => count === 0));
    assert.deepEqual(errors, []);
    await fs.writeFile(path.join(test.output, 'report.json'), JSON.stringify({
      desktop: true, mobile: true, urlRestore: true, images: true, download: true, settings: true,
      install: true, versionSelection: true, rollback: true, selfUpgrade: true, selfDisable: true,
      cliRegister: true, cliMetadata: true, localHelperAction: true, pairedDeviceTaskAndReport: true,
      first: test.first.revision, second: test.second.revision, errors,
    }, null, 2));
  } catch (error) {
    const pages = browser.contexts().flatMap(context => context.pages());
    const snapshots = [];
    for (const [index, current] of pages.entries()) {
      await current.screenshot({ path: path.join(test.output, `failure-${index}.png`) }).catch(() => {});
      snapshots.push({ url: current.url(), frames: await Promise.all(current.frames().map(async frame => {
        return frame.evaluate(() => ({ hash: location.hash, text: document.body.innerText.slice(0, 3000) })).catch(() => null);
      })) });
    }
    await fs.writeFile(path.join(test.output, 'failure-state.json'), JSON.stringify(snapshots, null, 2));
    await fs.writeFile(path.join(test.output, 'http-errors.json'), JSON.stringify(await Promise.all(failedResponses), null, 2));
    await fs.writeFile(path.join(test.output, 'errors.json'), JSON.stringify(errors, null, 2));
    throw error;
  } finally {
    await browser.close();
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
