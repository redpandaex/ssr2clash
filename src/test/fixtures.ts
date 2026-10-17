import type { AppView, OutputDetails } from "@/lib/types";

export function emptyView(): AppView {
	return {
		core: {
			settings: {
				profile: {
					sourceUrl: "",
					filterKeywords: [],
					excludeKeywords: [],
					enableUdp: true,
					preset: { kind: "builtin" },
				},
				port: 25501,
				allowLan: false,
				closeToTray: true,
				fetchProxy: "direct",
				theme: "system",
			},
			revision: 0,
			mode: "live",
			serverRunning: false,
			refreshing: false,
			subscriptionUrl: null,
			activeResult: null,
			lastRequestAt: null,
			lastError: null,
			servedStale: false,
			logs: [],
		},
		desktop: {
			runMode: "portable",
			dataDirectory: "D:\\SSR2Clash\\data",
			appVersion: "1.1.0",
			autostartEnabled: false,
			autostartError: null,
			startupError: null,
		},
	};
}
export function outputDetails(): OutputDetails {
	return {
		yaml: "proxies:\n  - name: 香港 01\nproxy-groups:\n  - name: Proxy\nrules:\n  - MATCH,Proxy\n",
		nodes: ["香港 01", "日本 01"].map((name, index) => ({
			name,
			type: "ssr",
			server: `node${index}.example.invalid`,
			port: 8388,
			cipher: "aes-256-cfb",
			password: "test-password",
			protocol: "auth_sha1_v4",
			obfs: "tls1.2_ticket_auth",
			udp: true,
		})),
		groups: Array.from({ length: 10 }, (_, index) => ({
			name: index === 0 ? "Proxy" : `Group ${index}`,
			type: "select",
			proxies: ["香港 01", "日本 01"],
		})),
		decodedPreview: "ssr://synthetic-only",
	};
}
export function populatedView(): AppView {
	const view = emptyView();
	view.core.settings.profile.sourceUrl =
		"https://source.example.invalid/sub?token=test-only";
	view.core.subscriptionUrl = "http://127.0.0.1:25501/sub/test-token.yaml";
	view.core.activeResult = {
		configRevision: 0,
		yaml: outputDetails().yaml,
		nodeCount: 2,
		groupCount: 10,
		invalidCount: 0,
		warnings: [],
		generatedAt: "2026-10-16T10:00:00Z",
	};
	return view;
}
