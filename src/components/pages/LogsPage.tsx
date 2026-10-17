import { displayTime } from "@/components/workbench/ServiceInspector";
import type { OutputDetails, RuntimeLogEntry } from "@/lib/types";

const levelNames: Record<string, string> = {
	info: "信息",
	warn: "警告",
	warning: "警告",
	error: "错误",
};

export function LogsPage({
	entries,
	output,
}: {
	entries: RuntimeLogEntry[];
	output: OutputDetails | null;
}) {
	return (
		<section className="logs-panel panel" aria-label="运行日志">
			<div className="logs-scroll">
				<table className="logs-table">
					<colgroup>
						<col className="log-time-col" />
						<col className="log-level-col" />
						<col />
					</colgroup>
					<thead>
						<tr>
							<th>时间</th>
							<th>级别</th>
							<th>内容</th>
						</tr>
					</thead>
					<tbody>
						{entries
							.slice(-100)
							.reverse()
							.map((entry) => (
								<tr key={entry.id} data-level={entry.level}>
									<td>
										<time
											dateTime={entry.at}
											title={new Date(entry.at).toLocaleString("zh-CN")}
										>
											{displayTime(entry.at)}
										</time>
									</td>
									<td>
										<span className="log-level">
											{levelNames[entry.level] ?? entry.level}
										</span>
									</td>
									<td>{entry.message}</td>
								</tr>
							))}
						{!entries.length && (
							<tr>
								<td className="logs-empty" colSpan={3}>
									暂无日志
								</td>
							</tr>
						)}
					</tbody>
				</table>
			</div>
			<details className="debug-preview">
				<summary>原始解析预览</summary>
				{output?.decodedPreview ? (
					<textarea
						aria-label="原始解析内容"
						readOnly
						wrap="off"
						value={output.decodedPreview}
					/>
				) : (
					<p className="muted">暂无内容</p>
				)}
			</details>
		</section>
	);
}
