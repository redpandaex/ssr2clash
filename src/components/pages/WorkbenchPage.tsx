import {
	AlertCircle,
	ArrowUpRight,
	LayoutDashboard,
	Play,
	Settings2,
	Snowflake,
	Square,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import {
	type OutputTab,
	OutputWorkspace,
} from "@/components/workbench/OutputWorkspace";
import { RuntimeLog } from "@/components/workbench/RuntimeLog";
import { ServiceInspector } from "@/components/workbench/ServiceInspector";
import { SubscriptionForm } from "@/components/workbench/SubscriptionForm";
import { useAppState } from "@/hooks/useAppState";
import { useSettingsDraft } from "@/hooks/useSettingsDraft";
import type { AppView } from "@/lib/types";

export function WorkbenchPage() {
	const state = useAppState();
	const theme = state.view?.core.settings.theme ?? "system";
	useEffect(() => {
		const media = window.matchMedia("(prefers-color-scheme: dark)");
		const apply = () => {
			const dark = theme === "dark" || (theme === "system" && media.matches);
			document.documentElement.classList.toggle("dark", dark);
			document.documentElement.style.colorScheme = dark ? "dark" : "light";
		};
		apply();
		media.addEventListener("change", apply);
		return () => media.removeEventListener("change", apply);
	}, [theme]);
	if (!state.view)
		return (
			<div className="startup-screen">
				<img src="/app-icon.svg" alt="" />
				<h1>SSR2Clash</h1>
				{state.error ? (
					<>
						<h2>暂时无法打开工作台</h2>
						<p role="alert">{state.error.message}</p>
						<button
							type="button"
							className="button primary"
							disabled={!!state.pending}
							onClick={() => void state.actions.retry()}
						>
							重新读取
						</button>
					</>
				) : (
					<>
						<p>正在读取本地设置…</p>
						<div
							className="startup-skeleton"
							role="status"
							aria-label="正在加载工作台"
						/>
					</>
				)}
			</div>
		);
	return <WorkbenchContent {...state} view={state.view} />;
}

function WorkbenchContent({
	view,
	output,
	pending,
	error,
	actions,
}: ReturnType<typeof useAppState> & { view: AppView }) {
	const { draft, setDraft, dirty } = useSettingsDraft(view.core.settings);
	const [settingsOpen, setSettingsOpen] = useState(false);
	const settingsOpener = useRef<HTMLElement | null>(null);
	const [tab, setTab] = useState<OutputTab>("nodes");
	const core = view.core;
	const frozen = core.mode === "snapshot";
	const busy = !!pending;
	function openSettings() {
		settingsOpener.current =
			document.activeElement instanceof HTMLElement
				? document.activeElement
				: null;
		setSettingsOpen(true);
	}
	async function refresh() {
		if (dirty && !(await actions.saveSettings(draft))) return;
		await actions.refresh();
	}
	const warning = core.lastError && core.activeResult;
	const operationError =
		error && (!warning || error.message !== core.lastError)
			? error.message
			: !warning
				? (core.lastError ?? view.desktop.startupError)
				: null;
	return (
		<div className="app-shell">
			<a className="skip-link" href="#main-content">
				跳到工作台
			</a>
			<aside className="sidebar">
				<div className="brand">
					<img src="/app-icon.svg" alt="SSR2Clash 图标" />
					<strong>SSR2Clash</strong>
					<span>订阅转换与本地托管</span>
				</div>
				<nav aria-label="主导航">
					<button
						type="button"
						className="active"
						aria-current="page"
						onClick={() => setTab("nodes")}
					>
						<LayoutDashboard size={18} />
						<span>工作台</span>
					</button>
					<button type="button" aria-label="打开设置" onClick={openSettings}>
						<Settings2 size={18} />
						<span>设置</span>
					</button>
				</nav>
				<div className="sidebar-bottom">
					<div>
						<span
							className={`status-dot ${core.serverRunning ? "blue" : ""}`}
						/>
						<span>
							{core.serverRunning ? "本地服务在线" : "本地服务未启动"}
						</span>
					</div>
					<p>
						{view.desktop.runMode === "portable" ? "便携模式" : "安装模式"}
						<span>{view.desktop.appVersion}</span>
					</p>
				</div>
			</aside>
			<main className="workbench" id="main-content">
				<header className="workbench-header">
					<div>
						<h1>工作台</h1>
						<p>订阅转换与本地托管</p>
					</div>
					<div className="header-actions">
						<span className={`mode-tag ${frozen ? "frozen" : ""}`}>
							{frozen ? (
								<Snowflake size={14} />
							) : (
								<span className="status-dot blue" />
							)}
							{frozen ? "快照模式" : "实时模式"}
						</span>
						<button
							type="button"
							className={`button ${core.serverRunning ? "secondary" : "primary"}`}
							disabled={
								busy ||
								(!core.serverRunning && !core.settings.profile.sourceUrl)
							}
							onClick={() =>
								void (core.serverRunning
									? actions.stopServer()
									: actions.startServer())
							}
						>
							{core.serverRunning ? <Square size={13} /> : <Play size={14} />}
							{core.serverRunning ? "停止服务" : "启动服务"}
						</button>
					</div>
				</header>
				<SubscriptionForm
					draft={draft}
					onChange={setDraft}
					frozen={frozen}
					busy={busy || core.refreshing}
					dirty={dirty}
					refreshing={pending === "refresh" || core.refreshing}
					onSave={() => void actions.saveSettings(draft)}
					onRefresh={() => void refresh()}
					onPreset={openSettings}
				/>
				{warning && (
					<div className="error-banner" role="alert">
						<AlertCircle size={18} />
						<div>
							<strong>上次更新失败，正在使用之前的配置</strong>
							<p>{core.lastError}</p>
						</div>
					</div>
				)}
				{operationError && (
					<div className="error-banner" role="alert">
						<AlertCircle size={18} />
						<p>{operationError}</p>
					</div>
				)}
				{frozen && (
					<div className="freeze-banner">
						<Snowflake size={15} />
						<span>
							当前输出已冻结。关闭右侧快照开关后，可修改订阅与分流设置。
						</span>
					</div>
				)}
				<div className="workspace-grid">
					<OutputWorkspace
						core={core}
						output={output}
						tab={tab}
						onTab={setTab}
					/>
					<ServiceInspector
						core={core}
						busy={busy}
						onSnapshot={(enabled) => void actions.setSnapshot(enabled)}
					/>
				</div>
				{!!core.activeResult?.warnings.length && (
					<details className="warnings-panel">
						<summary>
							{core.activeResult.invalidCount
								? `${core.activeResult.invalidCount} 个无效节点已跳过`
								: "转换提示"}
							<span>{core.activeResult.warnings.length} 条</span>
						</summary>
						<ul>
							{core.activeResult.warnings.map((message) => (
								<li key={message}>{message}</li>
							))}
						</ul>
					</details>
				)}
				<RuntimeLog entries={core.logs} output={output} />
				<footer className="workbench-footer">
					<span>配置在本机生成与保存</span>
					<a
						href="https://github.com/tindy2013/subconverter"
						target="_blank"
						rel="noreferrer"
					>
						默认分流源自 subconverter
						<ArrowUpRight size={12} />
					</a>
				</footer>
			</main>
			<SettingsDialog
				open={settingsOpen}
				onOpenChange={setSettingsOpen}
				view={view}
				draft={draft}
				onChange={setDraft}
				actions={actions}
				busy={busy}
				error={error}
				restoreFocus={() => settingsOpener.current?.focus()}
			/>
		</div>
	);
}
