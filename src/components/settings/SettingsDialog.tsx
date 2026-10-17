import * as Dialog from "@radix-ui/react-dialog";
import { FolderOpen, Settings2, X } from "lucide-react";
import { useState } from "react";
import { Switch } from "@/components/ui/switch";
import type { AppActions } from "@/hooks/useAppState";
import { chooseIni, openDataDirectory } from "@/lib/api/subscription";
import type { AppError, AppSettings, AppView, Theme } from "@/lib/types";

interface Props {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	view: AppView;
	draft: AppSettings;
	onChange: (draft: AppSettings) => void;
	actions: AppActions;
	busy: boolean;
	error: AppError | null;
	restoreFocus: () => void;
}
export function SettingsDialog({
	open,
	onOpenChange,
	view,
	draft,
	onChange,
	actions,
	busy,
	error,
	restoreFocus,
}: Props) {
	const [issue, setIssue] = useState("");
	const frozen = view.core.mode === "snapshot";
	const portValid =
		Number.isInteger(draft.port) && draft.port >= 1 && draft.port <= 65535;
	async function selectIni() {
		try {
			const path = await chooseIni();
			if (path)
				onChange({
					...draft,
					profile: { ...draft.profile, preset: { kind: "localIni", path } },
				});
		} catch {
			setIssue("无法打开文件选择器");
		}
	}
	return (
		<Dialog.Root open={open} onOpenChange={onOpenChange}>
			<Dialog.Portal>
				<Dialog.Overlay className="dialog-overlay" />
				<Dialog.Content
					className="settings-dialog"
					aria-describedby="settings-description"
					onCloseAutoFocus={(event) => {
						event.preventDefault();
						restoreFocus();
					}}
				>
					<header className="dialog-header">
						<div>
							<Dialog.Title>
								<Settings2 size={20} />
								设置
							</Dialog.Title>
							<Dialog.Description id="settings-description" className="sr-only">
								配置本地服务、应用行为和分流预设。
							</Dialog.Description>
						</div>
						<Dialog.Close className="icon-button" aria-label="关闭设置">
							<X size={19} />
						</Dialog.Close>
					</header>
					<div className="settings-scroll">
						<section className="settings-section">
							<h3>应用</h3>
							<div className="setting-row">
								<div>
									<label htmlFor="appearance-theme">外观主题</label>
								</div>
								<select
									id="appearance-theme"
									value={view.core.settings.theme}
									disabled={busy}
									onChange={(event) =>
										void actions.setTheme(event.target.value as Theme)
									}
								>
									<option value="system">跟随系统</option>
									<option value="light">浅色</option>
									<option value="dark">深色</option>
								</select>
							</div>
							<div className="setting-row">
								<div>
									<label htmlFor="autostart-setting">开机自启</label>
								</div>
								<Switch
									id="autostart-setting"
									checked={view.desktop.autostartEnabled}
									disabled={busy}
									onCheckedChange={(enabled) =>
										void actions.setAutostart(enabled)
									}
								/>
							</div>
							{view.desktop.autostartError && (
								<p className="inline-error">{view.desktop.autostartError}</p>
							)}
							<div className="setting-row">
								<div>
									<label htmlFor="close-to-tray">关闭窗口留在托盘</label>
								</div>
								<Switch
									id="close-to-tray"
									checked={draft.closeToTray}
									disabled={busy}
									onCheckedChange={(closeToTray) =>
										onChange({ ...draft, closeToTray })
									}
								/>
							</div>
						</section>
						<section className="settings-section">
							<h3>本地服务</h3>
							<div className="setting-row">
								<div>
									<label htmlFor="service-port">监听端口</label>
								</div>
								<input
									id="service-port"
									title={
										view.core.serverRunning ? "停止服务后可修改端口" : undefined
									}
									type="number"
									min={1}
									max={65535}
									required
									disabled={view.core.serverRunning || busy}
									value={draft.port || ""}
									onChange={(event) =>
										onChange({ ...draft, port: Number(event.target.value) })
									}
								/>
							</div>
							{!portValid && (
								<p className="inline-error">端口必须在 1 到 65535 之间</p>
							)}
							<div className="setting-row">
								<div>
									<label htmlFor="allow-lan">允许局域网访问</label>
								</div>
								<Switch
									id="allow-lan"
									title={
										view.core.serverRunning
											? "停止服务后可修改访问范围"
											: undefined
									}
									checked={draft.allowLan}
									disabled={view.core.serverRunning || busy}
									onCheckedChange={(allowLan) =>
										onChange({ ...draft, allowLan })
									}
								/>
							</div>
							<div className="setting-row">
								<div>
									<label htmlFor="fetch-proxy">获取订阅的代理</label>
								</div>
								<select
									id="fetch-proxy"
									value={draft.fetchProxy}
									disabled={busy}
									onChange={(event) =>
										onChange({
											...draft,
											fetchProxy: event.target
												.value as AppSettings["fetchProxy"],
										})
									}
								>
									<option value="direct">直连</option>
									<option value="system">系统代理</option>
								</select>
							</div>
						</section>
						<section className="settings-section">
							<h3>分组与规则</h3>
							<div className="field">
								<label htmlFor="preset-source">分流预设</label>
								<select
									id="preset-source"
									title={frozen ? "关闭冻结快照后可更改预设" : undefined}
									value={draft.profile.preset.kind}
									disabled={frozen || busy}
									onChange={(event) => {
										const kind = event.target.value;
										onChange({
											...draft,
											profile: {
												...draft.profile,
												preset:
													kind === "localIni"
														? { kind, path: "" }
														: kind === "remoteIni"
															? { kind, url: "" }
															: { kind: "builtin" },
											},
										});
									}}
								>
									<option value="builtin">内置默认分流</option>
									<option value="localIni">本地 INI 文件</option>
									<option value="remoteIni">远程 INI 地址</option>
								</select>
							</div>
							{draft.profile.preset.kind === "localIni" && (
								<div className="field preset-location">
									<label htmlFor="local-ini-path">INI 文件路径</label>
									<div>
										<input
											id="local-ini-path"
											value={draft.profile.preset.path}
											readOnly
											placeholder="选择本地 INI 文件"
										/>
										<button
											type="button"
											className="button secondary"
											disabled={frozen || busy}
											onClick={() => void selectIni()}
										>
											<FolderOpen size={15} />
											选择文件
										</button>
									</div>
								</div>
							)}
							{draft.profile.preset.kind === "remoteIni" && (
								<div className="field preset-location">
									<label htmlFor="remote-ini-url">INI 地址</label>
									<input
										id="remote-ini-url"
										type="url"
										placeholder="https://…/config.ini"
										disabled={frozen || busy}
										value={draft.profile.preset.url}
										onChange={(event) =>
											onChange({
												...draft,
												profile: {
													...draft.profile,
													preset: {
														kind: "remoteIni",
														url: event.target.value,
													},
												},
											})
										}
									/>
								</div>
							)}
						</section>
						<section className="settings-section data-location">
							<h3>数据与版本</h3>
							<span className="distribution-label">
								{view.desktop.runMode === "portable" ? "便携版" : "安装版"} ·
								SSR2Clash {view.desktop.appVersion}
							</span>
							<button
								type="button"
								className="data-directory-path mono"
								title="在文件管理器中打开"
								disabled={busy}
								onClick={() => {
									setIssue("");
									void openDataDirectory().catch(() =>
										setIssue("无法打开数据目录"),
									);
								}}
							>
								{view.desktop.dataDirectory}
							</button>
						</section>
						{(error || issue) && (
							<p className="inline-error" role="status">
								{error?.message ?? issue}
							</p>
						)}
					</div>
					<footer className="dialog-footer">
						<Dialog.Close className="button secondary">关闭</Dialog.Close>
						<button
							type="button"
							className="button primary"
							disabled={busy || !portValid}
							onClick={() => {
								void actions.saveSettings(draft).then((saved) => {
									if (saved) onOpenChange(false);
								});
							}}
						>
							保存设置
						</button>
					</footer>
				</Dialog.Content>
			</Dialog.Portal>
		</Dialog.Root>
	);
}
