export type Theme = "light" | "dark" | "system";
export type PresetSource =
	| { kind: "builtin" }
	| { kind: "localIni"; path: string }
	| { kind: "remoteIni"; url: string };
export interface ProfileConfig {
	sourceUrl: string;
	filterKeywords: string[];
	excludeKeywords: string[];
	enableUdp: boolean;
	preset: PresetSource;
}
export interface AppSettings {
	profile: ProfileConfig;
	port: number;
	allowLan: boolean;
	closeToTray: boolean;
	fetchProxy: "direct" | "system";
	theme: Theme;
}
export interface AppError {
	code: string;
	message: string;
}
export interface GeneratedConfig {
	configRevision: number;
	yaml: string;
	nodeCount: number;
	groupCount: number;
	invalidCount: number;
	warnings: string[];
	generatedAt: string;
}
export interface ClashProxy {
	name: string;
	type: string;
	server: string;
	port: number;
	cipher: string;
	password: string;
	protocol: string;
	obfs: string;
	udp: boolean;
	"obfs-param"?: string;
	"protocol-param"?: string;
}
export interface ClashProxyGroup {
	name: string;
	type: string;
	proxies: string[];
	url?: string;
	interval?: number;
	timeout?: number;
	tolerance?: number;
}
export interface OutputDetails {
	yaml: string;
	nodes: ClashProxy[];
	groups: ClashProxyGroup[];
	decodedPreview: string | null;
}
export interface RuntimeLogEntry {
	id: number;
	at: string;
	level: string;
	message: string;
}
export interface CoreStatus {
	settings: AppSettings;
	revision: number;
	mode: "live" | "snapshot";
	serverRunning: boolean;
	refreshing: boolean;
	subscriptionUrl: string | null;
	activeResult: GeneratedConfig | null;
	lastRequestAt: string | null;
	lastError: string | null;
	servedStale: boolean;
	logs: RuntimeLogEntry[];
}
export interface AppView {
	core: CoreStatus;
	desktop: {
		runMode: "portable" | "installed";
		dataDirectory: string;
		appVersion: string;
		autostartEnabled: boolean;
		autostartError: string | null;
		startupError: string | null;
	};
}
