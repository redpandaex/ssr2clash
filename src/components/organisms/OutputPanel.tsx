import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ActionBar } from "../molecules/ActionBar";

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

interface OutputPanelProps {
	code: string;
	nodeCount?: number;
	loading?: boolean;
	decodedContent?: string;
	ssrLinks?: string[];
	decodedSsrLinks?: DecodedSsrLink[];
	ssrNodes?: SsrNodeInfo[];
}

export function OutputPanel({
	code,
	nodeCount,
	loading,
	decodedContent,
	ssrLinks,
	decodedSsrLinks,
	ssrNodes,
}: OutputPanelProps) {
	return (
		<Card className="flex flex-col h-[calc(100vh-200px)]">
			<CardHeader className="flex-shrink-0">
				<div className="flex items-center justify-between">
					<CardTitle>输出预览</CardTitle>
					<div className="flex items-center gap-2">
						{nodeCount !== undefined && nodeCount > 0 && (
							<Badge variant="secondary">{nodeCount} 个节点</Badge>
						)}
						<ActionBar content={code} disabled={loading || !code} />
					</div>
				</div>
			</CardHeader>

			<CardContent className="flex-1 min-h-0">
				<Tabs defaultValue="yaml" className="h-full flex flex-col">
					<TabsList className="flex-shrink-0 grid grid-cols-5 w-full">
						<TabsTrigger value="yaml">Clash 配置</TabsTrigger>
						{decodedContent && (
							<TabsTrigger value="base64">Base64 解码</TabsTrigger>
						)}
						{ssrLinks && ssrLinks.length > 0 && (
							<TabsTrigger value="ssr-links">SSR 链接</TabsTrigger>
						)}
						{decodedSsrLinks && decodedSsrLinks.length > 0 && (
							<TabsTrigger value="ssr-decoded">SSR 解码</TabsTrigger>
						)}
						{ssrNodes && ssrNodes.length > 0 && (
							<TabsTrigger value="ssr-nodes">节点详情</TabsTrigger>
						)}
					</TabsList>

					{/* Clash YAML */}
					<TabsContent value="yaml" className="flex-1 min-h-0 mt-2">
						{code ? (
							<ScrollArea className="h-full w-full rounded-lg border">
								<pre className="p-4 font-mono text-xs whitespace-pre leading-relaxed">
									{code}
								</pre>
							</ScrollArea>
						) : (
							<div className="h-full flex items-center justify-center border rounded-lg bg-muted/20">
								<p className="text-sm text-muted-foreground">
									转换后的配置将显示在这里...
								</p>
							</div>
						)}
					</TabsContent>

					{/* Base64 解码内容 */}
					{decodedContent && (
						<TabsContent value="base64" className="flex-1 min-h-0 mt-2">
							<ScrollArea className="h-full w-full rounded-lg border">
								<pre className="p-4 font-mono text-xs whitespace-pre-wrap break-all leading-relaxed">
									{decodedContent}
								</pre>
							</ScrollArea>
						</TabsContent>
					)}

					{/* SSR 链接列表（编码） */}
					{ssrLinks && ssrLinks.length > 0 && (
						<TabsContent value="ssr-links" className="flex-1 min-h-0 mt-2">
							<ScrollArea className="h-full w-full rounded-lg border">
								<div className="p-4 space-y-3">
									{ssrLinks.map((link, index) => (
										<div
											// biome-ignore lint/suspicious/noArrayIndexKey: <i>Using index as key is acceptable here since the list is static and won't change.</i>
											key={index}
											className="p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
										>
											<div className="flex items-start gap-3">
												<Badge
													variant="outline"
													className="text-xs shrink-0 mt-0.5"
												>
													#{index + 1}
												</Badge>
												<div className="flex-1 min-w-0">
													<code className="text-xs font-mono break-all leading-relaxed block">
														{link}
													</code>
												</div>
											</div>
										</div>
									))}
								</div>
							</ScrollArea>
						</TabsContent>
					)}

					{/* SSR 链接解码后内容 */}
					{decodedSsrLinks && decodedSsrLinks.length > 0 && (
						<TabsContent value="ssr-decoded" className="flex-1 min-h-0 mt-2">
							<ScrollArea className="h-full w-full rounded-lg border">
								<div className="p-4 space-y-4">
									{decodedSsrLinks.map((item, index) => (
										// biome-ignore lint/suspicious/noArrayIndexKey: <i>Using index as key is acceptable here since the list is static and won't change.</i>
										<Card key={index} className="p-4">
											<div className="space-y-3">
												<div className="flex items-center justify-between">
													<Badge variant="outline" className="text-xs">
														#{index + 1}
													</Badge>
												</div>
												<Separator />
												<div className="space-y-2">
													<div>
														<p className="text-xs text-muted-foreground mb-1">
															原始链接:
														</p>
														<code className="text-xs font-mono break-all block bg-muted p-2 rounded">
															{item.original}
														</code>
													</div>
													<div>
														<p className="text-xs text-muted-foreground mb-1">
															解码后:
														</p>
														<code className="text-xs font-mono break-all block bg-muted p-2 rounded">
															{item.decoded}
														</code>
													</div>
												</div>
											</div>
										</Card>
									))}
								</div>
							</ScrollArea>
						</TabsContent>
					)}

					{/* SSR 节点详情 */}
					{ssrNodes && ssrNodes.length > 0 && (
						<TabsContent value="ssr-nodes" className="flex-1 min-h-0 mt-2">
							<ScrollArea className="h-full w-full rounded-lg border">
								<div className="p-4 space-y-3">
									{ssrNodes.map((node, index) => (
										<Card
											// biome-ignore lint/suspicious/noArrayIndexKey: <i>Using index as key is acceptable here since the list is static and won't change.</i>
											key={index}
											className="p-4 hover:bg-accent/50 transition-colors"
										>
											<div className="space-y-3">
												<div className="flex items-center justify-between">
													<h3 className="font-semibold text-sm truncate flex-1">
														{node.remarks || "未命名节点"}
													</h3>
													<Badge
														variant="outline"
														className="text-xs ml-2 shrink-0"
													>
														#{index + 1}
													</Badge>
												</div>
												<Separator />
												<div className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
													<div className="flex items-center gap-2">
														<span className="text-muted-foreground">
															服务器:
														</span>
														<span className="font-mono truncate">
															{node.server}
														</span>
													</div>
													<div className="flex items-center gap-2">
														<span className="text-muted-foreground">端口:</span>
														<span className="font-mono">{node.port}</span>
													</div>
													<div className="flex items-center gap-2">
														<span className="text-muted-foreground">协议:</span>
														<span className="font-mono">{node.protocol}</span>
													</div>
													<div className="flex items-center gap-2">
														<span className="text-muted-foreground">加密:</span>
														<span className="font-mono truncate">
															{node.method}
														</span>
													</div>
													<div className="flex items-center gap-2 col-span-2">
														<span className="text-muted-foreground">混淆:</span>
														<span className="font-mono truncate">
															{node.obfs}
														</span>
													</div>
												</div>
											</div>
										</Card>
									))}
								</div>
							</ScrollArea>
						</TabsContent>
					)}
				</Tabs>
			</CardContent>
		</Card>
	);
}
