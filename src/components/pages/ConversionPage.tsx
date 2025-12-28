import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";
import { useState } from "react";
import { toast } from "sonner";
import { InputPanel } from "../organisms/InputPanel";
import { OutputPanel } from "../organisms/OutputPanel";

interface ConvertConfig {
	filterKeywords: string[];
	excludeKeywords: string[];
	renameRules: Array<{ pattern: string; replacement: string }>;
	sortBy: string;
	enableUdp: boolean;
}

export function ConversionPage() {
	const [url, setUrl] = useState("");
	const [filterKeywords, setFilterKeywords] = useState<string[]>([]);
	const [excludeKeywords, setExcludeKeywords] = useState<string[]>([]);
	const [yaml, setYaml] = useState("");
	const [nodeCount, setNodeCount] = useState(0);
	const [loading, setLoading] = useState(false);

	const handleConvert = async () => {
		setLoading(true);
		const toastId = toast.loading("正在获取订阅...");

		try {
			// 1. 前端：获取订阅内容
			const response = await fetch(url, {
				method: "GET",
				headers: {
					"User-Agent": "ClashForWindows/0.20.39",
				},
			});

			if (!response.ok) {
				throw new Error(`HTTP ${response.status}: ${response.statusText}`);
			}

			const rawContent = await response.text();

			// 2. 前端：Base64 解码
			toast.loading("正在解析节点...", { id: toastId });
			const decodedContent = atob(rawContent.trim());

			// 3. 前端：提取 SSR 链接
			const ssrLinks = decodedContent
				.split("\n")
				.map((line) => line.trim())
				.filter((line) => line.startsWith("ssr://"));

			if (ssrLinks.length === 0) {
				throw new Error("未找到有效的 SSR 节点");
			}

			// 4. 后端：解析和转换
			toast.loading("正在转换配置...", { id: toastId });
			const config: ConvertConfig = {
				filterKeywords,
				excludeKeywords,
				renameRules: [],
				sortBy: "name",
				enableUdp: true,
			};

			const result = await invoke<string>("convert_ssr_to_clash", {
				ssrLinks,
				config,
			});

			setYaml(result);

			// 统计节点数量
			const count = (result.match(/- name:/g) || []).length;
			setNodeCount(count);

			toast.success(`转换成功！共 ${count} 个节点`, { id: toastId });
		} catch (error) {
			console.error(error);
			toast.error(error instanceof Error ? error.message : "转换失败", {
				id: toastId,
			});
		} finally {
			setLoading(false);
		}
	};

	return (
		<div className="min-h-screen bg-background p-6">
			<div className="max-w-7xl mx-auto space-y-6">
				{/* 页面标题 */}
				<div className="text-center space-y-2">
					<h1 className="text-4xl font-bold tracking-tight">
						SSR 订阅转换工具
					</h1>
					<p className="text-muted-foreground">
						一键转换 SSR 订阅为 Clash 配置
					</p>
				</div>

				{/* 主要内容区 - 双栏布局 */}
				<div className="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">
					{/* 左侧：输入面板 */}
					<InputPanel
						url={url}
						onUrlChange={setUrl}
						filterKeywords={filterKeywords}
						onFilterKeywordsChange={setFilterKeywords}
						excludeKeywords={excludeKeywords}
						onExcludeKeywordsChange={setExcludeKeywords}
						onConvert={handleConvert}
						loading={loading}
					/>

					{/* 右侧：输出面板 */}
					<OutputPanel code={yaml} nodeCount={nodeCount} loading={loading} />
				</div>
			</div>
		</div>
	);
}
