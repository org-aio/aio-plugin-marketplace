const sources = await dioxus.recv();
const blobs = new Set();
let disposed = false;
let total = 0;
const dispose = dioxus.recv().then(() => {
    disposed = true;
    for (const blob of blobs) URL.revokeObjectURL(blob);
    blobs.clear();
});
// 限制并发与总内存；取消后等待中的响应不得创建或泄漏 Blob。
let cursor = 0;
async function load() {
    while (!disposed && cursor < sources.length) {
        const source = sources[cursor++];
        try {
            const response = await window.aioPlugin.managementImage(source);
            if (disposed) return;
            if (response.status !== 200) throw new Error(`HTTP ${response.status}`);
            total += response.body.byteLength;
            if (total > 32 * 1024 * 1024) throw new Error('文档图片总大小超过 32 MiB');
            const type = response.headers.find(header => header.name.toLowerCase() === 'content-type')?.value;
            if (!type?.startsWith('image/')) throw new Error('响应不是图片');
            const blob = URL.createObjectURL(new Blob([response.body], { type }));
            blobs.add(blob);
            dioxus.send({ source, blob });
        } catch (error) {
            if (!disposed) dioxus.send({ source, error: `${source}：${error.message ?? error}` });
        }
    }
}
await Promise.all(Array.from({ length: Math.min(4, sources.length) }, load));
await dispose;
