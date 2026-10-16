import { Copy, Download, Link2, Snowflake } from "lucide-react";
import { useState } from "react";
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
}: {
	core: CoreStatus;
	busy: boolean;
	onSnapshot: (enabled: boolean) => void;
}) {
	const [feedback, setFeedback] = useState("");
	const frozen = core.mode === "snapshot";
	async function copy() {
		try {
			await copyText(core.subscriptionUrl ?? "");
			setFeedback("本地订阅链接已复制");
		} catch {
			setFeedback("复制失败，请选中链接手动复制");
		}
	}
	async function download() {
		try {
			if (core.activeResult && (await exportYaml(core.activeResult.yaml)))
				setFeedback("YAML 已导出");
		} catch {
			setFeedback("导出失败，请检查所选目录权限");
		}
	}
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
				<span className="loopback-label">仅本机</span>
			</div>
			<p className="muted service-intro">
				将此链接添加到 Clash Verge 的订阅中。
			</p>
			<div className="local-link">
				<input
					aria-label="本地订阅链接"
					readOnly
					value={core.subscriptionUrl ?? "保存原订阅后生成本地链接"}
					onFocus={(event) => event.target.select()}
				/>
				<button
					className="icon-button"
					type="button"
					disabled={!core.subscriptionUrl}
					aria-label="复制本地订阅链接"
					title="复制链接"
					onClick={() => void copy()}
				>
					<Copy size={16} />
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
				<p>
					{frozen
						? "直接返回已保存的 YAML，不请求原订阅。"
						: "实时模式：每次订阅请求都重新拉取。"}
				</p>
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
			{!core.serverRunning && core.subscriptionUrl && (
				<p className="service-note">启动服务后，Clash Verge 才能访问此链接。</p>
			)}
			<div className="inspector-bottom">
				<button
					type="button"
					className="button outline full-width"
					disabled={!core.activeResult}
					onClick={() => void download()}
				>
					<Download size={16} />
					导出 YAML
				</button>
				<p className="feedback" aria-live="polite">
					{feedback || "完整配置，包含节点、策略组与规则"}
				</p>
			</div>
		</aside>
	);
}
