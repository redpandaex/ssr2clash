import { FileText, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";

interface Base64PreviewProps {
	content: string;
	onClose: () => void;
}

export function Base64Preview({ content, onClose }: Base64PreviewProps) {
	const lines = content.split("\n").filter(Boolean);
	const ssrCount = lines.filter((line) => line.startsWith("ssr://")).length;

	return (
		<div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
			<Card className="w-full max-w-3xl max-h-[80vh] flex flex-col">
				<CardHeader>
					<div className="flex items-center justify-between">
						<div className="space-y-1">
							<CardTitle className="flex items-center gap-2">
								<FileText className="h-5 w-5" />
								Base64 解码内容
							</CardTitle>
							<div className="flex gap-2">
								<Badge variant="secondary">{ssrCount} 个 SSR 节点</Badge>
								<Badge variant="outline">{lines.length} 行</Badge>
							</div>
						</div>
						<Button variant="ghost" size="icon" onClick={onClose}>
							<X className="h-4 w-4" />
						</Button>
					</div>
				</CardHeader>
				<CardContent className="flex-1 min-h-0">
					<ScrollArea className="h-full border rounded-lg">
						<pre className="p-4 text-sm font-mono">{content || "(空内容)"}</pre>
					</ScrollArea>
				</CardContent>
			</Card>
		</div>
	);
}
