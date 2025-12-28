import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ActionBar } from "../molecules/ActionBar";
import { CodeViewer } from "../molecules/CodeViewer";

interface OutputPanelProps {
	code: string;
	nodeCount?: number;
	loading?: boolean;
}

export function OutputPanel({ code, nodeCount, loading }: OutputPanelProps) {
	return (
		<Card className="flex flex-col h-full">
			<CardHeader>
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
				<CodeViewer code={code} />
			</CardContent>
		</Card>
	);
}
