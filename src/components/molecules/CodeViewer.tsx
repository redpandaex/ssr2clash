import { FileCode } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Textarea } from "@/components/ui/textarea";

interface CodeViewerProps {
	code: string;
	language?: string;
	placeholder?: string;
}

export function CodeViewer({
	code,
	placeholder = "转换后的配置将显示在这里...",
}: CodeViewerProps) {
	return (
		<div className="h-full flex flex-col">
			{code ? (
				<ScrollArea className="h-full rounded-lg border">
					<Textarea
						value={code}
						readOnly
						className="min-h-125 font-mono text-sm resize-none border-0 focus-visible:ring-0"
					/>
				</ScrollArea>
			) : (
				<div className="h-full flex items-center justify-center border rounded-lg bg-muted/20">
					<div className="text-center space-y-3 p-8">
						<div className="flex justify-center">
							<div className="rounded-full bg-primary/10 p-4">
								<FileCode className="h-8 w-8 text-muted-foreground" />
							</div>
						</div>
						<p className="text-sm text-muted-foreground">{placeholder}</p>
					</div>
				</div>
			)}
		</div>
	);
}
