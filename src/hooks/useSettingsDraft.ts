import { useEffect, useRef, useState } from "react";
import type { AppSettings } from "@/lib/types";

// Reconcile fields independently, so a background theme/status update never erases an unsaved URL.
function reconcile<T extends object>(draft: T, previous: T, incoming: T): T {
	const result = { ...draft };
	for (const key of Object.keys(incoming) as (keyof T)[]) {
		if (JSON.stringify(draft[key]) === JSON.stringify(previous[key]))
			result[key] = incoming[key];
	}
	return result;
}
export function useSettingsDraft(settings: AppSettings) {
	const [draft, setDraft] = useState(settings);
	const previous = useRef(settings);
	useEffect(() => {
		const prior = previous.current;
		setDraft((current) => ({
			...reconcile(current, prior, settings),
			profile: reconcile(current.profile, prior.profile, settings.profile),
		}));
		previous.current = settings;
	}, [settings]);
	return {
		draft,
		setDraft,
		dirty: JSON.stringify(draft) !== JSON.stringify(settings),
	};
}
