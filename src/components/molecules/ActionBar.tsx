import { Check, Copy, Download } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

interface ActionBarProps {
	content: string;
	disabled?: boolean;
	onCopy?: () => void;
	onDownload?: () => void;
}

export function ActionBar({
	content,
	disabled,
	onCopy,
	onDownload,
}: ActionBarProps) {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		try {
			await navigator.clipboard.writeText(content);
			setCopied(true);
			toast.success("已复制到剪贴板");
			setTimeout(() => setCopied(false), 2000);
			onCopy?.();
		} catch (_error) {
			toast.error("复制失败");
		}
	};

	const handleDownload = () => {
		const blob = new Blob([content], { type: "text/yaml" });
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = `clash_${Date.now()}.yaml`;
		a.click();
		URL.revokeObjectURL(url);
		toast.success("文件已下载");
		onDownload?.();
	};

	return (
		<div className="flex items-center gap-2">
			<Button
				variant="outline"
				size="sm"
				onClick={handleCopy}
				disabled={disabled || !content}
			>
				{copied ? (
					<>
						<Check className="mr-2 h-4 w-4" />
						已复制
					</>
				) : (
					<>
						<Copy className="mr-2 h-4 w-4" />
						复制
					</>
				)}
			</Button>
			<Button
				variant="outline"
				size="sm"
				onClick={handleDownload}
				disabled={disabled || !content}
			>
				<Download className="mr-2 h-4 w-4" />
				下载
			</Button>
		</div>
	);
}
