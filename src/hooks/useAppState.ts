import { useCallback, useEffect, useRef, useState } from "react";
import { listenStateChanged, subscriptionApi } from "@/lib/api/subscription";
import type {
	AppError,
	AppSettings,
	AppView,
	OutputDetails,
	Theme,
} from "@/lib/types";

type Action =
	| "load"
	| "save"
	| "refresh"
	| "snapshot"
	| "server"
	| "autostart"
	| "theme";
function appError(error: unknown): AppError {
	if (
		error &&
		typeof error === "object" &&
		"message" in error &&
		typeof error.message === "string"
	) {
		return {
			code: "code" in error ? String(error.code) : "DesktopError",
			message: error.message,
		};
	}
	return { code: "DesktopError", message: "操作未能完成，请重试。" };
}

export function useAppState() {
	const [view, setView] = useState<AppView | null>(null);
	const [output, setOutput] = useState<OutputDetails | null>(null);
	const [pending, setPending] = useState<Action | null>("load");
	const [error, setError] = useState<AppError | null>(null);
	const current = useRef<AppView | null>(null);
	const outputYaml = useRef<string | null>(null);
	const readSequence = useRef(0);
	const mounted = useRef(false);

	const apply = useCallback(async (next: AppView) => {
		if (!mounted.current) return;
		current.current = next;
		setView(next);
		const yaml = next.core.activeResult?.yaml ?? null;
		if (!yaml) {
			outputYaml.current = null;
			setOutput(null);
			return;
		}
		if (outputYaml.current === yaml) return;
		outputYaml.current = yaml;
		setOutput(null);
		try {
			const details = await subscriptionApi.getOutputDetails();
			if (mounted.current && outputYaml.current === details.yaml)
				setOutput(details);
		} catch (failure) {
			if (mounted.current && outputYaml.current === yaml) {
				outputYaml.current = null;
				setError(appError(failure));
			}
		}
	}, []);

	const reload = useCallback(async () => {
		const sequence = ++readSequence.current;
		const next = await subscriptionApi.getState();
		if (sequence === readSequence.current) await apply(next);
	}, [apply]);

	useEffect(() => {
		mounted.current = true;
		let disposed = false;
		let unlisten: (() => void) | undefined;
		void reload()
			.catch((failure) => {
				if (mounted.current) setError(appError(failure));
			})
			.finally(() => {
				if (mounted.current) setPending(null);
			});
		void listenStateChanged(() => {
			if (!disposed)
				void reload().catch((failure) => {
					if (mounted.current) setError(appError(failure));
				});
		})
			.then((stop) => {
				if (!disposed) unlisten = stop;
				else stop();
			})
			.catch((failure) => {
				if (mounted.current) setError(appError(failure));
			});
		return () => {
			disposed = true;
			mounted.current = false;
			++readSequence.current;
			unlisten?.();
		};
	}, [reload]);

	const run = useCallback(
		async (action: Action, operation: () => Promise<AppView>) => {
			setPending(action);
			setError(null);
			try {
				const next = await operation();
				// Read the authoritative state after a mutation; a concurrent HTTP update may have finished meanwhile.
				await reload().catch(() => apply(next));
				return true;
			} catch (failure) {
				if (mounted.current) setError(appError(failure));
				await reload().catch(() => {});
				return false;
			} finally {
				if (mounted.current) setPending(null);
			}
		},
		[apply, reload],
	);

	const actions = {
		retry: () => run("load", subscriptionApi.getState),
		saveSettings: (settings: AppSettings) =>
			run("save", () => subscriptionApi.saveSettings(settings)),
		refresh: () => run("refresh", subscriptionApi.refreshSubscription),
		setSnapshot: (enabled: boolean) =>
			run("snapshot", () => subscriptionApi.setSnapshotEnabled(enabled)),
		startServer: () => run("server", subscriptionApi.startServer),
		stopServer: () => run("server", subscriptionApi.stopServer),
		setAutostart: (enabled: boolean) =>
			run("autostart", () => subscriptionApi.setAutostartEnabled(enabled)),
		setTheme: (theme: Theme) =>
			run("theme", () =>
				subscriptionApi.saveSettings({
					...(current.current as AppView).core.settings,
					theme,
				}),
			),
	};
	return { view, output, pending, error, actions };
}
export type AppActions = ReturnType<typeof useAppState>["actions"];
