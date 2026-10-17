import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { afterEach, expect, it, vi } from "vitest";
import { exportYaml } from "./subscription";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeTextFile: vi.fn() }));

afterEach(() => {
	vi.useRealTimers();
	vi.clearAllMocks();
});

it("names each YAML export using the local date at the time of export", async () => {
	vi.useFakeTimers();
	vi.mocked(save).mockResolvedValue("D:/exports/config.yaml");
	vi.setSystemTime(new Date(2026, 0, 2, 0, 5));
	await exportYaml("MATCH,DIRECT\n");
	expect(save).toHaveBeenLastCalledWith({
		defaultPath: "2026-01-02.yaml",
		filters: [{ name: "Clash 配置", extensions: ["yaml", "yml"] }],
	});
	expect(writeTextFile).toHaveBeenLastCalledWith(
		"D:/exports/config.yaml",
		"MATCH,DIRECT\n",
	);

	vi.setSystemTime(new Date(2026, 9, 17, 23, 55));
	await exportYaml("MATCH,Proxy\n");
	expect(save).toHaveBeenLastCalledWith({
		defaultPath: "2026-10-17.yaml",
		filters: [{ name: "Clash 配置", extensions: ["yaml", "yml"] }],
	});
	expect(writeTextFile).toHaveBeenLastCalledWith(
		"D:/exports/config.yaml",
		"MATCH,Proxy\n",
	);
});
