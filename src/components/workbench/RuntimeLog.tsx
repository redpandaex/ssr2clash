import { ChevronDown, FileText } from "lucide-react";
import type { OutputDetails, RuntimeLogEntry } from "@/lib/types";
import { displayTime } from "./ServiceInspector";

export function RuntimeLog({
	entries,
	output,
}: {
	entries: RuntimeLogEntry[];
	output: OutputDetails | null;
}) {
	return (
		<details className="runtime-panel panel">
			<summary>
				<FileText size={16} />
				<span>运行记录</span>
				<span className="runtime-count">
					{entries.length
						? `最近 ${Math.min(entries.length, 100)} 条`
						: "暂无记录"}
				</span>
				<ChevronDown size={15} />
			</summary>
			<div className="runtime-body">
				{entries.length ? (
					<ol>
						{entries
							.slice(-100)
							.reverse()
							.map((entry) => (
								<li key={entry.id} data-level={entry.level}>
									<time>{displayTime(entry.at)}</time>
									<span>{entry.message}</span>
								</li>
							))}
					</ol>
				) : (
					<p className="muted">更新订阅或启动服务后，可在这里查看运行状态。</p>
				)}
				<details className="debug-preview">
					<summary>原始解析预览</summary>
					{output?.decodedPreview ? (
						<>
							<p className="muted">包含订阅原文，请勿公开分享。</p>
							<textarea
								aria-label="原始解析内容"
								readOnly
								wrap="off"
								value={output.decodedPreview}
							/>
						</>
					) : (
						<p className="muted">
							实时更新后可查看原始内容，重开应用后不保留此预览。
						</p>
					)}
				</details>
			</div>
		</details>
	);
}
