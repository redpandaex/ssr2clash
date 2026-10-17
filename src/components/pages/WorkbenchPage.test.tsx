import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { subscriptionApi } from "@/lib/api/subscription";
import type { AppView } from "@/lib/types";
import { emptyView, outputDetails, populatedView } from "@/test/fixtures";
import { WorkbenchPage } from "./WorkbenchPage";

vi.mock("@/lib/api/subscription", () => ({
	subscriptionApi: {
		getState: vi.fn(),
		getOutputDetails: vi.fn(),
		saveSettings: vi.fn(),
		refreshSubscription: vi.fn(),
		setSnapshotEnabled: vi.fn(),
		startServer: vi.fn(),
		stopServer: vi.fn(),
		setAutostartEnabled: vi.fn(),
	},
	listenStateChanged: vi.fn(async () => () => {}),
	copyText: vi.fn(),
	openDataDirectory: vi.fn(),
	exportYaml: vi.fn(),
	chooseIni: vi.fn(),
}));
let state: AppView;
beforeEach(() => {
	vi.clearAllMocks();
	state = emptyView();
	vi.mocked(subscriptionApi.getState).mockImplementation(async () =>
		structuredClone(state),
	);
	vi.mocked(subscriptionApi.getOutputDetails).mockResolvedValue(
		outputDetails(),
	);
	vi.mocked(subscriptionApi.saveSettings).mockImplementation(
		async (settings) => {
			state.core.settings = settings;
			return structuredClone(state);
		},
	);
});

describe("subscription workbench", () => {
	it("disables refresh for an empty source and freeze without a generated result", async () => {
		render(<WorkbenchPage />);
		expect(
			await screen.findByRole("button", { name: "立即更新" }),
		).toBeDisabled();
		expect(screen.getByRole("switch", { name: "冻结快照" })).toBeDisabled();
		expect(screen.getByText("内置默认分流")).toBeVisible();
	});

	it("counts structured nodes separately from strategy groups", async () => {
		state = populatedView();
		render(<WorkbenchPage />);
		expect(await screen.findByText("2 个节点")).toBeVisible();
		expect(screen.getByText("10 个策略组")).toBeVisible();
		expect(await screen.findByText("node0.example.invalid")).toBeVisible();
	});

	it("keeps the old result and persistent error after refresh fails", async () => {
		state = populatedView();
		vi.mocked(subscriptionApi.refreshSubscription).mockImplementation(
			async () => {
				state.core.lastError = "原订阅暂时不可用";
				throw { code: "HttpStatus", message: "原订阅暂时不可用" };
			},
		);
		render(<WorkbenchPage />);
		await userEvent.click(
			await screen.findByRole("button", { name: "立即更新" }),
		);
		expect(
			await screen.findByText("上次更新失败，正在使用之前的配置"),
		).toBeVisible();
		expect(screen.getByText("2 个节点")).toBeVisible();
		expect(screen.getByLabelText("原订阅地址")).toHaveValue(
			state.core.settings.profile.sourceUrl,
		);
	});

	it("freezes editing of output settings while leaving settings accessible", async () => {
		state = populatedView();
		state.core.mode = "snapshot";
		render(<WorkbenchPage />);
		expect(await screen.findByLabelText("原订阅地址")).toBeDisabled();
		expect(screen.getByRole("textbox", { name: "包含关键词" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "立即更新" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "打开设置" })).toBeEnabled();
	});

	it("shows the stopped state after stopping the actual service", async () => {
		state = populatedView();
		state.core.serverRunning = true;
		vi.mocked(subscriptionApi.stopServer).mockImplementation(async () => {
			state.core.serverRunning = false;
			return structuredClone(state);
		});
		render(<WorkbenchPage />);
		await userEvent.click(
			await screen.findByRole("button", { name: "停止服务" }),
		);
		expect(
			await screen.findByRole("button", { name: "启动服务" }),
		).toBeEnabled();
		expect(screen.getByText("服务已停止")).toBeVisible();
	});

	it("preserves an unsaved address when persistence fails", async () => {
		state = populatedView();
		vi.mocked(subscriptionApi.saveSettings).mockRejectedValue({
			code: "StorageWrite",
			message: "磁盘写入失败",
		});
		render(<WorkbenchPage />);
		const source = await screen.findByLabelText("原订阅地址");
		await userEvent.clear(source);
		await userEvent.type(source, "https://changed.example.invalid/new");
		await userEvent.click(screen.getByRole("button", { name: "保存订阅" }));
		await waitFor(() =>
			expect(subscriptionApi.saveSettings).toHaveBeenCalled(),
		);
		expect(await screen.findByText("磁盘写入失败")).toBeVisible();
		expect(source).toHaveValue("https://changed.example.invalid/new");
		expect(subscriptionApi.refreshSubscription).not.toHaveBeenCalled();
	});

	it("changes only the theme when switching appearance", async () => {
		state = populatedView();
		state.core.serverRunning = true;
		const original = structuredClone(state.core.settings);
		render(<WorkbenchPage />);
		await userEvent.click(
			await screen.findByRole("button", { name: "打开设置" }),
		);
		await userEvent.selectOptions(
			screen.getByRole("combobox", { name: "外观主题" }),
			"dark",
		);
		await waitFor(() =>
			expect(subscriptionApi.saveSettings).toHaveBeenCalledWith({
				...original,
				theme: "dark",
			}),
		);
		expect(subscriptionApi.stopServer).not.toHaveBeenCalled();
	});

	it("does not hide a new save failure behind an earlier refresh warning", async () => {
		state = populatedView();
		state.core.lastError = "之前的订阅更新失败";
		vi.mocked(subscriptionApi.saveSettings).mockRejectedValue({
			code: "StorageWrite",
			message: "当前磁盘无法写入",
		});
		render(<WorkbenchPage />);
		const source = await screen.findByLabelText("原订阅地址");
		await userEvent.type(source, "/changed");
		await userEvent.click(screen.getByRole("button", { name: "保存订阅" }));
		expect(await screen.findByText("当前磁盘无法写入")).toBeVisible();
		expect(source).toHaveValue(
			`${state.core.settings.profile.sourceUrl}/changed`,
		);
	});

	it("shows the upstream failure when no configuration has been generated yet", async () => {
		state.core.settings.profile.sourceUrl =
			"https://source.example.invalid/sub";
		vi.mocked(subscriptionApi.refreshSubscription).mockImplementation(
			async () => {
				state.core.lastError = "原订阅返回了无效内容";
				throw { code: "InvalidSubscription", message: "原订阅返回了无效内容" };
			},
		);
		render(<WorkbenchPage />);
		await userEvent.click(
			await screen.findByRole("button", { name: "立即更新" }),
		);
		expect(await screen.findByText("原订阅返回了无效内容")).toBeVisible();
	});
});
