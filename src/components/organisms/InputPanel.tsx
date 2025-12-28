import { Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { FilterSection } from "../molecules/FilterSection";
import { UrlInput } from "../molecules/UrlInput";

interface InputPanelProps {
	url: string;
	onUrlChange: (url: string) => void;
	filterKeywords: string[];
	onFilterKeywordsChange: (keywords: string[]) => void;
	excludeKeywords: string[];
	onExcludeKeywordsChange: (keywords: string[]) => void;
	onConvert: () => void;
	loading?: boolean;
	disabled?: boolean;
}

export function InputPanel({
	url,
	onUrlChange,
	filterKeywords,
	onFilterKeywordsChange,
	excludeKeywords,
	onExcludeKeywordsChange,
	onConvert,
	loading,
	disabled,
}: InputPanelProps) {
	return (
		<Card>
			<CardHeader>
				<CardTitle>订阅配置</CardTitle>
			</CardHeader>
			<CardContent className="space-y-6">
				{/* URL 输入 */}
				<UrlInput value={url} onChange={onUrlChange} disabled={disabled} />

				{/* 配置选项 */}
				<Tabs defaultValue="filter" className="w-full">
					<TabsList className="grid w-full grid-cols-2">
						<TabsTrigger value="filter">过滤选项</TabsTrigger>
						<TabsTrigger value="advanced">高级选项</TabsTrigger>
					</TabsList>

					<TabsContent value="filter" className="space-y-4">
						<FilterSection
							label="包含关键词"
							placeholder="例如：香港, 美国"
							keywords={filterKeywords}
							onChange={onFilterKeywordsChange}
							disabled={disabled}
						/>
						<FilterSection
							label="排除关键词"
							placeholder="例如：过期, 测试"
							keywords={excludeKeywords}
							onChange={onExcludeKeywordsChange}
							disabled={disabled}
						/>
					</TabsContent>

					<TabsContent value="advanced" className="space-y-4">
						<div className="p-4 rounded-lg bg-muted/50 text-sm text-muted-foreground">
							高级选项开发中...
						</div>
					</TabsContent>
				</Tabs>

				{/* 转换按钮 */}
				<Button
					className="w-full"
					onClick={onConvert}
					disabled={disabled || loading || !url}
				>
					{loading ? (
						<>正在转换...</>
					) : (
						<>
							<Zap className="mr-2 h-4 w-4" />
							开始转换
						</>
					)}
				</Button>
			</CardContent>
		</Card>
	);
}
