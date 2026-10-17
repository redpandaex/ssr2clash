import { ChevronDown, Eye, EyeOff, Link2, RefreshCw, Save } from "lucide-react";
import { useState } from "react";
import { Switch } from "@/components/ui/switch";
import type { AppSettings } from "@/lib/types";

interface Props {
	draft: AppSettings;
	onChange: (value: AppSettings) => void;
	frozen: boolean;
	busy: boolean;
	dirty: boolean;
	refreshing: boolean;
	onSave: () => void;
	onRefresh: () => void;
	onPreset: () => void;
}
export function SubscriptionForm({
	draft,
	onChange,
	frozen,
	busy,
	dirty,
	refreshing,
	onSave,
	onRefresh,
	onPreset,
}: Props) {
	const [reveal, setReveal] = useState(false);
	const [include, setInclude] = useState(
		draft.profile.filterKeywords.join(", "),
	);
	const [exclude, setExclude] = useState(
		draft.profile.excludeKeywords.join(", "),
	);
	const keywords = (value: string) =>
		value
			.split(/[,，\n]/)
			.map((item) => item.trim())
			.filter(Boolean);
	const presetName =
		draft.profile.preset.kind === "builtin"
			? "内置默认分流"
			: draft.profile.preset.kind === "localIni"
				? "本地 INI 预设"
				: "远程 INI 预设";
	return (
		<section className="subscription-panel panel" aria-label="订阅配置">
			<div className="field-heading">
				<label htmlFor="source-url">原订阅地址</label>
			</div>
			<form
				className="source-row"
				onSubmit={(event) => {
					event.preventDefault();
					onRefresh();
				}}
			>
				<div className="source-input-wrap">
					<Link2 size={17} aria-hidden="true" />
					<input
						id="source-url"
						aria-label="原订阅地址"
						type={reveal ? "text" : "password"}
						autoComplete="off"
						spellCheck={false}
						placeholder="粘贴原始 SSR 订阅链接"
						value={draft.profile.sourceUrl}
						disabled={frozen || busy}
						onChange={(event) =>
							onChange({
								...draft,
								profile: { ...draft.profile, sourceUrl: event.target.value },
							})
						}
					/>
					<button
						type="button"
						className="icon-button"
						aria-label={reveal ? "隐藏原订阅地址" : "显示原订阅地址"}
						title={reveal ? "隐藏地址" : "显示地址"}
						onClick={() => setReveal(!reveal)}
					>
						{reveal ? <EyeOff size={16} /> : <Eye size={16} />}
					</button>
				</div>
				<button
					type="button"
					className="button secondary save-source"
					aria-label="保存订阅"
					title="仅保存订阅设置"
					disabled={!dirty || frozen || busy}
					onClick={onSave}
				>
					<Save size={15} />
					<span>保存</span>
				</button>
				<button
					type="submit"
					className="button primary"
					disabled={!draft.profile.sourceUrl.trim() || frozen || busy}
				>
					<RefreshCw size={15} className={refreshing ? "spinning" : ""} />
					{refreshing ? "正在更新" : "立即更新"}
				</button>
			</form>
			<div className="filter-row">
				<div className="field">
					<label htmlFor="include-keywords">包含关键词</label>
					<input
						id="include-keywords"
						placeholder="例如：香港，日本"
						value={include}
						disabled={frozen || busy}
						onChange={(event) => {
							setInclude(event.target.value);
							onChange({
								...draft,
								profile: {
									...draft.profile,
									filterKeywords: keywords(event.target.value),
								},
							});
						}}
					/>
				</div>
				<div className="field">
					<label htmlFor="exclude-keywords">排除关键词</label>
					<input
						id="exclude-keywords"
						placeholder="例如：到期，流量"
						value={exclude}
						disabled={frozen || busy}
						onChange={(event) => {
							setExclude(event.target.value);
							onChange({
								...draft,
								profile: {
									...draft.profile,
									excludeKeywords: keywords(event.target.value),
								},
							});
						}}
					/>
				</div>
				<div className="field">
					<span className="field-label">分组与规则</span>
					<button
						type="button"
						className="preset-button"
						onClick={onPreset}
						disabled={frozen || busy}
					>
						<span>{presetName}</span>
						<ChevronDown size={15} />
					</button>
				</div>
				<div className="udp-control">
					<label htmlFor="enable-udp">UDP</label>
					<Switch
						id="enable-udp"
						checked={draft.profile.enableUdp}
						disabled={frozen || busy}
						onCheckedChange={(enableUdp) =>
							onChange({ ...draft, profile: { ...draft.profile, enableUdp } })
						}
					/>
				</div>
			</div>
		</section>
	);
}
