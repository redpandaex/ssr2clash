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

interface SsrNodeInfo {
	remarks: string;
	server: string;
	port: number;
	protocol: string;
	method: string;
	obfs: string;
}

interface DecodedSsrLink {
	original: string;
	decoded: string;
}

export function ConversionPage() {
	const [url, setUrl] = useState("");
	const [filterKeywords, setFilterKeywords] = useState<string[]>([]);
	const [excludeKeywords, setExcludeKeywords] = useState<string[]>([]);
	const [yaml, setYaml] = useState("");
	const [nodeCount, setNodeCount] = useState(0);
	const [loading, setLoading] = useState(false);

	// 解码和解析内容
	const [decodedContent, setDecodedContent] = useState("");
	const [ssrLinks, setSsrLinks] = useState<string[]>([]);
	const [decodedSsrLinks, setDecodedSsrLinks] = useState<DecodedSsrLink[]>([]);
	const [ssrNodes, setSsrNodes] = useState<SsrNodeInfo[]>([]);

	// 解码 URL Safe Base64
	const decodeUrlSafeBase64 = (str: string): string => {
		try {
			// 1. 将 URL Safe Base64 转换为标准 Base64
			// - 替换 - 为 +
			// - 替换 _ 为 /
			let base64 = str.replace(/-/g, "+").replace(/_/g, "/");

			// 2. 添加缺失的 padding (=)
			const padding = base64.length % 4;
			if (padding > 0) {
				base64 += "=".repeat(4 - padding);
			}

			// 3. 标准 Base64 解码
			return atob(base64);
		} catch (error) {
			console.error("Base64 解码失败:", error);
			return "(解码失败)";
		}
	};

	// 解码单个 SSR 链接
	const decodeSsrLink = (link: string): string => {
		try {
			// 去掉 ssr:// 前缀
			const encoded = link.replace("ssr://", "");
			// URL Safe Base64 解码
			const decoded = decodeUrlSafeBase64(encoded);
			return decoded;
		} catch (error) {
			console.error("SSR 链接解码失败:", error);
			return "(解码失败)";
		}
	};

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

			// 2. 前端：Base64 解码（标准 Base64，通常不含 - 和 _）
			toast.loading("正在解析节点...", { id: toastId });
			const decoded = atob(rawContent.trim());
			setDecodedContent(decoded);

			// 3. 前端：提取 SSR 链接
			const links = decoded
				.split("\n")
				.map((line) => line.trim())
				.filter((line) => line.startsWith("ssr://"));

			if (links.length === 0) {
				throw new Error("未找到有效的 SSR 节点");
			}

			setSsrLinks(links); // 保存原始 SSR 链接

			// 4. 解码每个 SSR 链接（支持 URL Safe Base64）
			const decodedLinks: DecodedSsrLink[] = links
				.slice(0, 100)
				.map((link) => ({
					original: link,
					decoded: decodeSsrLink(link),
				}));
			setDecodedSsrLinks(decodedLinks);

			// 5. 解析 SSR 节点信息（用于详情预览）
			const parsedNodes = await Promise.all(
				links.slice(0, 100).map(async (link) => {
					try {
						const nodeJson = await invoke<string>("parse_ssr_link_command", {
							link,
						});
						return JSON.parse(nodeJson) as SsrNodeInfo;
					} catch {
						return null;
					}
				}),
			);
			setSsrNodes(parsedNodes.filter((n): n is SsrNodeInfo => n !== null));

			// 6. 后端：解析和转换
			toast.loading("正在转换配置...", { id: toastId });
			const config: ConvertConfig = {
				filterKeywords,
				excludeKeywords,
				renameRules: [],
				sortBy: "name",
				enableUdp: true,
			};

			const result = await invoke<string>("convert_ssr_to_clash", {
				ssrLinks: links,
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

					{/* 右侧：输出面板（集成 5 个 Tabs） */}
					<OutputPanel
						code={yaml}
						nodeCount={nodeCount}
						loading={loading}
						decodedContent={decodedContent}
						ssrLinks={ssrLinks}
						decodedSsrLinks={decodedSsrLinks}
						ssrNodes={ssrNodes}
					/>
				</div>
			</div>
		</div>
	);
}
