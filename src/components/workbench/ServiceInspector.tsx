import {
	Check,
	Copy,
	Download,
	Link2,
	RefreshCw,
	Snowflake,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Switch } from "@/components/ui/switch";
import { copyText, exportYaml } from "@/lib/api/subscription";
import type { CoreStatus } from "@/lib/types";

export function displayTime(value: string | null | undefined) {
	return value
		? new Date(value).toLocaleTimeString("zh-CN", {
				hour12: false,
				hour: "2-digit",
				minute: "2-digit",
				second: "2-digit",
			})
		: "尚无";
}
export function ServiceInspector({
	core,
	busy,
	onSnapshot,
	onRefreshAddresses,
}: {
	core: CoreStatus;
	busy: boolean;
	onSnapshot: (enabled: boolean) => void;
	onRefreshAddresses: () => void;
}) {
	const [issue, setIssue] = useState("");
	const [success, setSuccess] = useState<{
		kind: "local" | "lan" | "export";
		value?: string;
	} | null>(null);
	useEffect(() => {
		if (!success) return;
		const timer = setTimeout(() => setSuccess(null), 2000);
		return () => clearTimeout(timer);
	}, [success]);
	const frozen = core.mode === "snapshot";
	async function copy(value: string, kind: "local" | "lan") {
		setIssue("");
		setSuccess(null);
		try {
			await copyText(value);
			setSuccess({ kind, value });
		} catch {
			setIssue("复制失败，请选中链接手动复制");
		}
	}
	async function download() {
		setIssue("");
		setSuccess(null);
		try {
			if (core.activeResult && (await exportYaml(core.activeResult.yaml)))
				setSuccess({ kind: "export" });
		} catch {
			setIssue("导出失败，请检查所选目录权限");
		}
	}
	const copied =
		(success?.kind === "local" || success?.kind === "lan") &&
		success.value === core.subscriptionUrl;
	return (
		<aside className="service-panel panel" aria-label="本地订阅服务">
			<div className="inspector-title">
				<Link2 size={20} />
				<h2>本地订阅</h2>
			</div>
			<div
				className={`service-status ${core.serverRunning ? "is-running" : ""}`}
			>
				<span className="status-dot" />
				{core.serverRunning ? "服务运行中" : "服务已停止"}
				<span className="loopback-label">
					{core.settings.allowLan ? "局域网" : "仅本机"}
				</span>
			</div>
			<div className="local-link">
				<input
					aria-label="本地订阅链接"
					readOnly
					value={
						core.subscriptionUrl ??
						(!core.settings.profile.sourceUrl
							? "保存原订阅后生成订阅链接"
							: core.settings.allowLan
								? "未检测到可用的局域网地址"
								: "尚无订阅链接")
					}
					onFocus={(event) => event.target.select()}
				/>
				{core.settings.allowLan && (
					<button
						type="button"
						className="icon-button"
						aria-label="刷新局域网地址"
						title="刷新地址"
						disabled={busy}
						onClick={onRefreshAddresses}
					>
						<RefreshCw size={14} />
					</button>
				)}
				<button
					className="icon-button"
					type="button"
					disabled={!core.subscriptionUrl}
					aria-label="复制本地订阅链接"
					title={copied ? "已复制" : "复制链接"}
					onClick={() =>
						void copy(
							core.subscriptionUrl ?? "",
							core.settings.allowLan ? "lan" : "local",
						)
					}
				>
					{copied ? <Check size={16} /> : <Copy size={16} />}
				</button>
			</div>
			<div className={`snapshot-control ${frozen ? "is-frozen" : ""}`}>
				<div>
					<span className="snapshot-label">
						<Snowflake size={16} />
						冻结快照
					</span>
					<Switch
						aria-label="冻结快照"
						checked={frozen}
						disabled={!core.activeResult || busy}
						onCheckedChange={onSnapshot}
					/>
				</div>
			</div>
			<dl className="service-metadata">
				<div>
					<dt>生成时间</dt>
					<dd>{displayTime(core.activeResult?.generatedAt)}</dd>
				</div>
				<div>
					<dt>最近订阅请求</dt>
					<dd>{displayTime(core.lastRequestAt)}</dd>
				</div>
				<div>
					<dt>本地端口</dt>
					<dd className="mono">{core.settings.port}</dd>
				</div>
			</dl>
			<div className="inspector-bottom">
				<button
					type="button"
					className="button outline full-width"
					disabled={!core.activeResult}
					onClick={() => void download()}
				>
					{success?.kind === "export" ? (
						<Check size={16} />
					) : (
						<Download size={16} />
					)}
					{success?.kind === "export" ? "已导出" : "导出 YAML"}
				</button>
				{issue && (
					<p className="inline-error" role="alert">
						{issue}
					</p>
				)}
				<span className="sr-only" role="status">
					{success?.kind === "local"
						? "本地订阅链接已复制"
						: success?.kind === "lan"
							? "局域网订阅链接已复制"
							: success?.kind === "export"
								? "YAML 已导出"
								: ""}
				</span>
			</div>
		</aside>
	);
}
