const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const net = require('node:net');
const { spawn, execFileSync } = require('node:child_process');
const { chromium } = require(process.env.SSR2CLASH_PLAYWRIGHT_MODULE || 'playwright');
const root = process.cwd();
const workspace = path.join(root, 'src-tauri/target/desktop-smoke');
const testDir = path.join(workspace, `desktop test 中文 ${Date.now()}`);
const exe = path.join(testDir, 'ssr2clash.exe');
const wait = (ms) => new Promise(resolve => setTimeout(resolve, ms));
const children = [];
const browsers = [];
let hits = 0;
let activeApp;
let registeredByTest = false;

async function until(action, message, timeout = 20000) {
    const end = Date.now() + timeout;
    let last;
    while (Date.now() < end) {
        try { const value = await action(); if (value) return value; } catch (error) { last = error; }
        await wait(100);
    }
    throw new Error(`${message}: ${last?.message || 'timed out'}`);
}
async function freePort() {
    const server = net.createServer();
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const port = server.address().port;
    await new Promise(resolve => server.close(resolve));
    return port;
}
async function launch(args = []) {
    const debugPort = await freePort();
    const child = spawn(exe, args, { cwd: workspace, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
        WEBVIEW2_USER_DATA_FOLDER: path.join(testDir, 'webview-test'),
    }});
    children.push(child);
    child.stdout.on('data', data => process.stdout.write(`[app] ${data}`));
    child.stderr.on('data', data => process.stderr.write(`[app] ${data}`));
    await until(async () => { const response = await fetch(`http://127.0.0.1:${debugPort}/json/version`); return response.ok; }, 'WebView CDP did not start', 30000);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
    browsers.push(browser);
    const page = await until(() => browser.contexts().flatMap(context => context.pages()).find(page => page.url().includes('localhost:1420')), 'main WebView missing').catch(async error => {
        console.error('Observed CDP target URLs:', browser.contexts().flatMap(context => context.pages()).map(page => page.url()));
        for (const page of browser.contexts().flatMap(context => context.pages())) { console.error((await page.locator('body').innerText().catch(() => '')).slice(0, 1000)); }
        console.error('Vite HTTP check', await fetch('http://localhost:1420').then(response => response.status).catch(error => error.message));
        throw error;
    });
    await page.waitForFunction(() => !!window.__TAURI_INTERNALS__, { timeout: 15000 });
    return { child, page, browser, invoke: (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args }) };
}
async function quit(app) {
    void app.invoke('quit_application').catch(() => {});
    await until(() => app.child.exitCode !== null, 'application did not quit');
    await Promise.race([app.browser.close().catch(() => {}), wait(1500)]);
}

(async () => {
    await fs.mkdir(testDir, { recursive: true });
    await fs.copyFile(path.join(root, 'src-tauri/target/debug/ssr2clash.exe'), exe);
    await fs.writeFile(path.join(testDir, 'portable.flag'), '');
    const input = await fs.readFile(path.join(root, 'src-tauri/tests/fixtures/subconverter/input.txt'));
    const origin = http.createServer((_request, response) => { hits++; response.end(input); });
    await new Promise(resolve => origin.listen(0, '127.0.0.1', resolve));
    children.push({ kill: () => origin.close() });
    const vite = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), '--host', '127.0.0.1'], { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    vite.stdout.on('data', data => process.stdout.write(`[vite] ${data}`));
    vite.stderr.on('data', data => process.stderr.write(`[vite] ${data}`));
    vite.on('exit', (code, signal) => console.log('Vite helper exited:', code, signal));
    children.push(vite);
    await until(async () => (await fetch('http://localhost:1420')).ok, 'Vite did not start');
    let app = activeApp = await launch();
    let view = await app.invoke('get_state');
    assert.equal(view.desktop.runMode, 'portable');
    assert.equal(path.resolve(view.desktop.dataDirectory), path.join(testDir, 'data'));
    assert.equal(view.core.serverRunning, false);
    assert.equal(view.core.settings.profile.sourceUrl, '');
    const settings = view.core.settings;
    settings.profile.sourceUrl = `http://127.0.0.1:${origin.address().port}/source`;
    settings.port = await freePort();
    await app.invoke('save_settings', { settings });
    view = await app.invoke('refresh_subscription');
    assert.equal(view.core.activeResult.nodeCount, 2);
    assert.equal(view.core.activeResult.groupCount, 10);
    view = await app.invoke('start_server');
    assert.equal(view.core.serverRunning, true);
    const url = view.core.subscriptionUrl;
    await fetch(url);
    const yaml = await (await fetch(url)).text();
    assert.equal(hits, 3);
    await app.invoke('set_snapshot_enabled', { enabled: true });
    console.log('PASS native IPC, structured counts, per-GET upstream fetch, frozen mode');

    await app.page.evaluate(async () => {
        window.testEvents = 0;
        await window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
            event: 'app-state-changed', target: {kind: 'Any'},
            handler: window.__TAURI_INTERNALS__.transformCallback(() => window.testEvents++),
        });
    });
    settings.theme = 'dark';
    await app.invoke('save_settings', { settings });
    await until(() => app.page.evaluate(() => window.testEvents > 0), 'state event missing');
    const closeScript = `Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class AppWindow { [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l); }'; $handle = (Get-Process -Id ${app.child.pid}).MainWindowHandle; if ($handle -eq 0) { throw 'No main window handle' }; [AppWindow]::PostMessage($handle, 16, [IntPtr]::Zero, [IntPtr]::Zero)`;
    execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', closeScript], { windowsHide: true });
    await until(async () => !(await app.invoke('plugin:window|is_visible', { label: 'main' })), 'window did not hide');
    assert.equal(await (await fetch(url)).text(), yaml);
    assert.equal(hits, 3);
    const duplicate = spawn(exe, [], { cwd: workspace, windowsHide: true, stdio: 'ignore' });
    children.push(duplicate);
    await until(() => duplicate.exitCode !== null, 'duplicate process did not exit');
    await until(() => app.invoke('plugin:window|is_visible', { label: 'main' }), 'manual second launch did not show window');
    console.log('PASS close-to-tray keeps HTTP alive, state events, second launch restores existing window');

    const runKey = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Run';
    const existing = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `if (Get-ItemProperty -LiteralPath '${runKey}' -Name SSR2Clash -ErrorAction SilentlyContinue) { 'exists' }`], { windowsHide: true, encoding: 'utf8' }).trim();
    if (!existing) {
        view = await app.invoke('set_autostart_enabled', { enabled: true });
        registeredByTest = true;
        assert.equal(view.desktop.autostartEnabled, true);
        const command = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `[Console]::OutputEncoding = [Text.UTF8Encoding]::new(); (Get-ItemProperty -LiteralPath '${runKey}').SSR2Clash`], { windowsHide: true, encoding: 'utf8' }).trim();
        assert.equal(command, `"${exe}" --autostart`);
        view = await app.invoke('set_autostart_enabled', { enabled: false });
        registeredByTest = false;
        assert.equal(view.desktop.autostartEnabled, false);
        console.log('PASS real current-user autostart register/unregister, quoted Unicode and space path');
    } else { console.log('SKIP autostart mutation: an existing registration was preserved'); }

    await quit(app);
    console.log('After quit, Vite status:', await fetch('http://localhost:1420').then(response => response.status).catch(error => error.message), 'helper exit:', vite.exitCode);
    await until(async () => { try { await fetch(url); return false; } catch { return true; } }, 'quit did not release listener');
    app = activeApp = await launch(['--autostart']);
    view = await app.invoke('get_state');
    assert.equal(view.core.serverRunning, true);
    assert.equal(view.core.mode, 'snapshot');
    assert.equal(view.core.subscriptionUrl, url);
    assert.equal(await app.invoke('plugin:window|is_visible', { label: 'main' }), false);
    assert.equal(await (await fetch(url)).text(), yaml);
    assert.equal(hits, 3);
    console.log('PASS explicit quit releases port; simulated login launch restores frozen service while hidden');
    await app.invoke('stop_server');
    await quit(app);
    app = activeApp = await launch();
    view = await app.invoke('get_state');
    assert.equal(view.core.serverRunning, false);
    assert.equal(view.core.subscriptionUrl, url);
    await quit(app);
    console.log('PASS manual stop persists across restart');
    const statePath = path.join(testDir, 'data/state.json');
    const originalState = await fs.readFile(statePath);
    await fs.writeFile(statePath, '{broken-test-state');
    app = activeApp = await launch(['--autostart']);
    const startupError = await app.page.evaluate(async () => {
        try { await window.__TAURI_INTERNALS__.invoke('get_state'); return null; } catch (error) { return error; }
    });
    assert.ok(startupError.code);
    assert.ok(startupError.message.includes('数据目录'));
    assert.equal(await app.invoke('plugin:window|is_visible', { label: 'main' }), true);
    assert.equal(await fs.readFile(statePath, 'utf8'), '{broken-test-state');
    await quit(app);
    await fs.writeFile(statePath, originalState);
    console.log('PASS corrupt state remains untouched and forces visible startup error');
})().catch(error => { console.error(error); process.exitCode = 1; }).finally(async () => {
    if (registeredByTest && activeApp?.child.exitCode === null) {
        await activeApp.invoke('set_autostart_enabled', { enabled: false }).catch(error => console.error('Test autostart cleanup failed', error));
    }
    for (const child of children.reverse()) { try { child.kill(); } catch {} }
    for (const browser of browsers) { await Promise.race([browser.close().catch(() => {}), wait(1500)]); }
});
