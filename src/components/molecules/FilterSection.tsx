import { X } from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface FilterSectionProps {
	label: string;
	placeholder: string;
	keywords: string[];
	onChange: (keywords: string[]) => void;
	disabled?: boolean;
}

export function FilterSection({
	label,
	placeholder,
	keywords,
	onChange,
	disabled,
}: FilterSectionProps) {
	const [inputValue, setInputValue] = useState("");

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter" && inputValue.trim()) {
			e.preventDefault();
			if (!keywords.includes(inputValue.trim())) {
				onChange([...keywords, inputValue.trim()]);
			}
			setInputValue("");
		}
	};

	const removeKeyword = (keyword: string) => {
		onChange(keywords.filter((k) => k !== keyword));
	};

	return (
		<div className="space-y-2">
			<Label className="text-sm font-medium">{label}</Label>
			<Input
				placeholder={placeholder}
				value={inputValue}
				onChange={(e) => setInputValue(e.target.value)}
				onKeyDown={handleKeyDown}
				disabled={disabled}
			/>
			{keywords.length > 0 && (
				<div className="flex flex-wrap gap-2 pt-1">
					{keywords.map((keyword) => (
						<Badge key={keyword} variant="secondary" className="gap-1">
							{keyword}
							<button
								type="button"
								onClick={() => removeKeyword(keyword)}
								className="ml-1 hover:text-destructive"
								disabled={disabled}
							>
								<X className="h-3 w-3" />
							</button>
						</Badge>
					))}
				</div>
			)}
			<p className="text-xs text-muted-foreground">按 Enter 添加关键词</p>
		</div>
	);
}
