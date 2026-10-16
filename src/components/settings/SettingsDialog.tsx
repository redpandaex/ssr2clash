import * as Dialog from "@radix-ui/react-dialog";
import { FolderOpen, Settings2, X } from "lucide-react";
import { useState } from "react";
import { Switch } from "@/components/ui/switch";
import type { AppActions } from "@/hooks/useAppState";
import { chooseIni, copyText } from "@/lib/api/subscription";
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
							<Dialog.Description id="settings-description">
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
									<p>浅色、深色或跟随系统。</p>
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
									<p>登录 Windows 后在托盘中启动。</p>
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
									<p>服务继续运行，可从托盘菜单退出。</p>
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
									<p>
										{view.core.serverRunning
											? "停止服务后可修改端口。"
											: "仅监听 127.0.0.1，改端口后需更新 Clash 中的链接。"}
									</p>
								</div>
								<input
									id="service-port"
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
									<label htmlFor="fetch-proxy">获取订阅的代理</label>
									<p>系统代理不可用时可切回直连。</p>
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
							<p className="section-description">
								默认使用 subconverter 默认分流，也可导入常见 INI 预设。
							</p>
							{frozen && (
								<p className="frozen-note">关闭冻结快照后可更改预设。</p>
							)}
							<div className="field">
								<label htmlFor="preset-source">分流预设</label>
								<select
									id="preset-source"
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
							<p className="settings-footnote">
								支持分组、规则列表与相对导入。使用脚本或模板语言的预设会提示不支持。
							</p>
						</section>
						<section className="settings-section data-location">
							<h3>数据与版本</h3>
							<span className="distribution-label">
								{view.desktop.runMode === "portable" ? "便携版" : "安装版"} ·
								SSR2Clash {view.desktop.appVersion}
							</span>
							<p className="mono" title={view.desktop.dataDirectory}>
								{view.desktop.dataDirectory}
							</p>
							<button
								type="button"
								className="text-button"
								onClick={() => {
									void copyText(view.desktop.dataDirectory)
										.then(() => setIssue("数据目录已复制"))
										.catch(() => setIssue("复制失败，请手动复制目录"));
								}}
							>
								复制数据目录
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
