import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { createServer } from "vite";

// Headless UI checks use synthetic subscriptions. They do not open the desktop app.
const target = path.resolve("output/playwright");
await mkdir(target, { recursive: true });
const vite = await createServer({
	server: { host: "127.0.0.1", port: 0, strictPort: false },
	clearScreen: false,
});
await vite.listen();
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
	const { emptyView, populatedView, outputDetails } = await vite.ssrLoadModule(
		"/src/test/fixtures.ts",
	);
	const config = await readFile(
		"src-tauri/tests/fixtures/subconverter/upstream.yaml",
		"utf8",
	);
	const origin = `http://127.0.0.1:${vite.httpServer.address().port}`;
	async function scenario(name, width, height, view, setup) {
		const page = await browser.newPage({
			viewport: { width, height },
			reducedMotion: "reduce",
		});
		const failures = [];
		page.on("pageerror", (error) => failures.push(error.message));
		const details = outputDetails();
		details.yaml = config;
		if (view.core.activeResult) view.core.activeResult.yaml = config;
		const api = `
            let state = ${JSON.stringify(view)};
            const details = ${JSON.stringify(details)};
            const handlers = new Set();
            const clone = () => structuredClone(state);
            window.testSetState = next => { state = next; for(const handler of handlers) handler(); };
            window.testGetState = clone;
            export const subscriptionApi = {
                getState: async () => clone(), getOutputDetails: async () => structuredClone(details),
                saveSettings: async settings => { state.core.settings = settings; return clone(); },
                refreshSubscription: async () => clone(),
                setSnapshotEnabled: async enabled => { state.core.mode = enabled ? 'snapshot' : 'live'; return clone(); },
                startServer: async () => { state.core.serverRunning = true; return clone(); },
                stopServer: async () => { state.core.serverRunning = false; return clone(); },
                setAutostartEnabled: async enabled => { state.desktop.autostartEnabled = enabled; return clone(); }
            };
            export const listenStateChanged = async handler => { handlers.add(handler); return () => handlers.delete(handler); };
            export const copyText = async value => { window.testClipboard = value; };
            export const exportYaml = async value => { window.testExport = value; return true; };
            export const chooseIni = async () => 'D:/test/config.ini';
        `;
		await page.route("**/src/lib/api/subscription.ts*", (route) =>
			route.fulfill({ contentType: "application/javascript", body: api }),
		);
		await page.goto(origin);
		await page.getByRole("heading", { name: "工作台", exact: true }).waitFor();
		await page.evaluate(() => document.fonts.ready);
		if (setup) await setup(page);
		assert.equal(
			await page.evaluate(
				() => document.documentElement.scrollWidth <= innerWidth,
			),
			true,
			`${name}: horizontal overflow`,
		);
		assert.deepEqual(failures, [], `${name}: browser errors`);
		await page.screenshot({
			path: path.join(target, `${name}.png`),
			fullPage: true,
		});
		console.log(`PASS ${name}, ${width}×${height}`);
		await page.close();
	}
	const populated = () => {
		const view = populatedView();
		view.core.serverRunning = true;
		return view;
	};
	await scenario(
		"workbench-light-1200",
		1200,
		800,
		populated(),
		async (page) => {
			await page.getByText("node0.example.invalid").waitFor();
			await page.getByRole("button", { name: "复制本地订阅链接" }).click();
			assert.equal(
				await page.evaluate(() => window.testClipboard),
				populated().core.subscriptionUrl,
			);
			await page.getByRole("button", { name: "导出 YAML" }).click();
			assert.equal(await page.evaluate(() => window.testExport), config);
		},
	);
	await scenario("workbench-light-1000", 1000, 700, populated());
	const dark = populated();
	dark.core.settings.theme = "dark";
	await scenario("workbench-dark", 1200, 800, dark);
	const frozen = populated();
	frozen.core.mode = "snapshot";
	await scenario("workbench-frozen", 1000, 700, frozen, async (page) =>
		assert.equal(
			await page.getByLabel("原订阅地址", { exact: true }).isDisabled(),
			true,
		),
	);
	const failure = populated();
	failure.core.lastError = "原订阅暂时不可用，请稍后更新";
	failure.core.servedStale = true;
	await scenario("workbench-failure", 1200, 800, failure);
	await scenario("workbench-empty", 1200, 800, emptyView());
	const loading = emptyView();
	loading.core.refreshing = true;
	await scenario("workbench-loading", 1000, 700, loading);
	await scenario("workbench-settings", 1200, 800, populated(), async (page) => {
		await page.getByRole("button", { name: "打开设置" }).click();
		await page.getByRole("dialog").waitFor();
		await page.getByRole("combobox", { name: "外观主题" }).selectOption("dark");
		await page.waitForFunction(() =>
			document.documentElement.classList.contains("dark"),
		);
		assert.equal(
			await page.evaluate(() =>
				document.documentElement.classList.contains("dark"),
			),
			true,
		);
		await page.keyboard.press("Escape");
		await page.getByRole("dialog").waitFor({ state: "hidden" });
		assert.equal(
			await page
				.getByRole("button", { name: "打开设置" })
				.evaluate((element) => element === document.activeElement),
			true,
		);
		await page.getByRole("button", { name: "打开设置" }).click();
	});
	await scenario("workbench-yaml", 1200, 800, populated(), async (page) => {
		await page.getByRole("tab", { name: "YAML", exact: true }).click();
		assert.equal(await page.getByLabel("完整 YAML 配置").inputValue(), config);
		await page.getByRole("button", { name: "复制 YAML", exact: true }).click();
		assert.equal(await page.evaluate(() => window.testClipboard), config);
	});
	await scenario(
		"workbench-source-reveal",
		1200,
		800,
		populated(),
		async (page) => {
			const source = page.getByLabel("原订阅地址", { exact: true });
			const address = "https://source.example.invalid/pasted-subscription";
			await source.fill("");
			await source.pressSequentially(address);
			const cdp = await page.context().newCDPSession(page);
			try {
				await cdp.send("DOM.enable");
				await cdp.send("CSS.enable");
				const { root } = await cdp.send("DOM.getDocument", {
					depth: -1,
					pierce: true,
				});
				function findReveal(node) {
					if (node.attributes?.includes("password-reveal")) return node;
					for (const child of [
						...(node.children ?? []),
						...(node.shadowRoots ?? []),
					]) {
						const found = findReveal(child);
						if (found) return found;
					}
				}
				const reveal = findReveal(root);
				assert.ok(reveal, "Edge created its native password reveal control");
				const { computedStyle } = await cdp.send(
					"CSS.getComputedStyleForNode",
					{
						nodeId: reveal.nodeId,
					},
				);
				assert.equal(
					computedStyle.find((property) => property.name === "display")?.value,
					"none",
					"native reveal control must be hidden when the custom toggle is present",
				);
			} finally {
				await cdp.detach();
			}
			await page
				.getByRole("button", { name: "显示原订阅地址", exact: true })
				.click();
			assert.equal(await source.getAttribute("type"), "text");
			assert.equal(await source.inputValue(), address);
			await page
				.getByRole("button", { name: "隐藏原订阅地址", exact: true })
				.click();
			assert.equal(await source.getAttribute("type"), "password");
			assert.equal(await source.inputValue(), address);
		},
	);
} finally {
	await browser.close();
	await vite.close();
}
