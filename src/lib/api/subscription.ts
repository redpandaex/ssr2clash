import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import type { AppSettings, AppView, OutputDetails } from "@/lib/types";

const call = <T>(command: string, args?: Record<string, unknown>) => {
	if (!isTauri())
		return Promise.reject({
			code: "DesktopRequired",
			message: "请使用桌面应用打开工作台，浏览器预览无法访问桌面服务。",
		});
	return invoke<T>(command, args);
};
export const subscriptionApi = {
	getState: () => call<AppView>("get_state"),
	getOutputDetails: () => call<OutputDetails>("get_output_details"),
	saveSettings: (settings: AppSettings) =>
		call<AppView>("save_settings", { settings }),
	refreshSubscription: () => call<AppView>("refresh_subscription"),
	setSnapshotEnabled: (enabled: boolean) =>
		call<AppView>("set_snapshot_enabled", { enabled }),
	startServer: () => call<AppView>("start_server"),
	stopServer: () => call<AppView>("stop_server"),
	setAutostartEnabled: (enabled: boolean) =>
		call<AppView>("set_autostart_enabled", { enabled }),
};
export const listenStateChanged = (handler: () => void) =>
	isTauri() ? listen("app-state-changed", handler) : Promise.resolve(() => {});
export const copyText = (value: string) => navigator.clipboard.writeText(value);
export async function exportYaml(yaml: string) {
	const now = new Date();
	const date = [
		now.getFullYear(),
		String(now.getMonth() + 1).padStart(2, "0"),
		String(now.getDate()).padStart(2, "0"),
	].join("-");
	const path = await save({
		defaultPath: `${date}.yaml`,
		filters: [{ name: "Clash 配置", extensions: ["yaml", "yml"] }],
	});
	if (path) await writeTextFile(path, yaml);
	return !!path;
}
export async function chooseIni() {
	const path = await open({
		title: "选择 subconverter INI 预设",
		multiple: false,
		filters: [{ name: "INI 预设", extensions: ["ini"] }],
	});
	return typeof path === "string" ? path : null;
}
