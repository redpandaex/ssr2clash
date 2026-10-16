import * as Tabs from "@radix-ui/react-tabs";
import {
	Check,
	ChevronDown,
	ChevronLeft,
	ChevronRight,
	Copy,
	FileCode2,
	Network,
	Search,
} from "lucide-react";
import { useState } from "react";
import { copyText } from "@/lib/api/subscription";
import type { CoreStatus, OutputDetails } from "@/lib/types";

export type OutputTab = "nodes" | "groups" | "yaml";
export function OutputWorkspace({
	core,
	output,
	tab,
	onTab,
}: {
	core: CoreStatus;
	output: OutputDetails | null;
	tab: OutputTab;
	onTab: (tab: OutputTab) => void;
}) {
	const [query, setQuery] = useState("");
	const [page, setPage] = useState(0);
	const [copied, setCopied] = useState(false);
	const [copyError, setCopyError] = useState("");
	const result = core.activeResult;
	const nodes =
		output?.nodes.filter((node) =>
			`${node.name} ${node.server}`.toLowerCase().includes(query.toLowerCase()),
		) ?? [];
	const pages = Math.max(1, Math.ceil(nodes.length / 50));
	const currentPage = Math.min(page, pages - 1);
	async function copy() {
		if (!result) return;
		try {
			await copyText(result.yaml);
			setCopied(true);
			setCopyError("");
		} catch {
			setCopyError("复制失败，可选中 YAML 手动复制");
		}
	}
	return (
		<section
			className="output-panel panel"
			id="output-workspace"
			tabIndex={-1}
			aria-label="转换结果"
		>
			<Tabs.Root
				value={tab}
				onValueChange={(value) => onTab(value as OutputTab)}
			>
				<div className="output-toolbar">
					<Tabs.List className="output-tabs" aria-label="输出视图">
						<Tabs.Trigger value="nodes">节点</Tabs.Trigger>
						<Tabs.Trigger value="groups">策略组</Tabs.Trigger>
						<Tabs.Trigger value="yaml">YAML</Tabs.Trigger>
					</Tabs.List>
					<div className="output-counts">
						{result ? (
							<>
								<span>{result.nodeCount} 个节点</span>
								<span className="count-divider">·</span>
								<span>{result.groupCount} 个策略组</span>
							</>
						) : (
							<span>等待生成</span>
						)}
					</div>
				</div>
				{!result ? (
					<div
						className={`output-empty ${core.refreshing ? "is-loading" : ""}`}
						role="status"
					>
						<span className="empty-icon">
							<Network size={28} />
						</span>
						<h2>{core.refreshing ? "正在生成配置" : "从一条订阅开始"}</h2>
						<p>
							{core.refreshing
								? "正在获取节点并生成分组与规则，请稍候。"
								: "粘贴原订阅并更新，即可查看节点与完整 Clash 配置。"}
						</p>
						{core.refreshing && (
							<div className="skeleton-lines">
								<span />
								<span />
								<span />
							</div>
						)}
					</div>
				) : (
					<>
						<Tabs.Content value="nodes" className="output-content">
							<div className="node-tools">
								<span>可用节点</span>
								<div className="node-search">
									<Search size={14} />
									<input
										aria-label="搜索节点"
										placeholder="搜索名称或地址"
										value={query}
										onChange={(event) => {
											setQuery(event.target.value);
											setPage(0);
										}}
									/>
								</div>
							</div>
							{!output ? (
								<div className="output-loading" role="status">
									正在读取节点详情…
								</div>
							) : (
								<div className="table-viewport">
									<table className="nodes-table">
										<colgroup>
											<col className="name-col" />
											<col className="protocol-col" />
											<col className="server-col" />
											<col className="port-col" />
										</colgroup>
										<thead>
											<tr>
												<th>节点</th>
												<th>类型</th>
												<th>地址</th>
												<th>端口</th>
											</tr>
										</thead>
										<tbody>
											{nodes
												.slice(currentPage * 50, (currentPage + 1) * 50)
												.map((node) => (
													<tr key={node.name}>
														<td title={node.name}>
															<span className="node-symbol">
																<Network size={13} />
															</span>
															{node.name}
														</td>
														<td>
															<span className="protocol-tag">
																{node.type.toUpperCase()}
															</span>
														</td>
														<td
															className="mono server-cell"
															title={node.server}
														>
															{node.server}
														</td>
														<td className="mono">{node.port}</td>
													</tr>
												))}
										</tbody>
									</table>
									{nodes.length === 0 && (
										<p className="no-matches">没有匹配的节点</p>
									)}
								</div>
							)}
							<div className="table-footer">
								<span>
									{query
										? `${nodes.length} 个匹配结果`
										: "节点信息来自最近一次成功更新"}
								</span>
								{pages > 1 && (
									<div className="pagination">
										<button
											type="button"
											className="icon-button"
											aria-label="上一页节点"
											disabled={currentPage === 0}
											onClick={() => setPage(currentPage - 1)}
										>
											<ChevronLeft size={14} />
										</button>
										<span>
											{currentPage + 1} / {pages}
										</span>
										<button
											type="button"
											className="icon-button"
											aria-label="下一页节点"
											disabled={currentPage + 1 >= pages}
											onClick={() => setPage(currentPage + 1)}
										>
											<ChevronRight size={14} />
										</button>
									</div>
								)}
							</div>
						</Tabs.Content>
						<Tabs.Content value="groups" className="output-content">
							<div className="group-list">
								{output?.groups.map((group) => (
									<details className="group-item" key={group.name}>
										<summary>
											<Network size={16} />
											<strong>{group.name}</strong>
											<span className="group-kind">{group.type}</span>
											<span className="group-count">
												{group.proxies.length} 项
											</span>
											<ChevronDown size={14} />
										</summary>
										<div className="group-members">
											{group.proxies.map((name) => (
												<span key={name}>{name}</span>
											))}
										</div>
										{group.interval && (
											<p className="group-meta">
												测速间隔 {group.interval} 秒
												{group.tolerance ? ` · 容差 ${group.tolerance} ms` : ""}
											</p>
										)}
									</details>
								))}
							</div>
						</Tabs.Content>
						<Tabs.Content value="yaml" className="output-content">
							<div className="yaml-tools">
								<span>
									<FileCode2 size={15} />
									subscription.yaml
								</span>
								<button
									type="button"
									className="text-button"
									onClick={() => void copy()}
								>
									{copied ? <Check size={14} /> : <Copy size={14} />}
									{copied ? "已复制" : "复制 YAML"}
								</button>
							</div>
							{copyError && (
								<p className="inline-error" role="alert">
									{copyError}
								</p>
							)}
							<textarea
								className="yaml-preview"
								aria-label="完整 YAML 配置"
								readOnly
								spellCheck={false}
								wrap="off"
								value={result.yaml}
							/>
						</Tabs.Content>
					</>
				)}
			</Tabs.Root>
		</section>
	);
}
