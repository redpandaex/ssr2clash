# SSR2Clash 单订阅服务与桌面重做 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现具备完整 Clash 分组规则、本地实时／快照订阅、自启与全新 UI 的 Windows 工具，同时交付便携 ZIP 和可自选安装路径的安装包。

**Architecture:** Rust 转换核心供 Tauri 命令和 Axum 本地服务共同调用。带配置版本的状态服务负责持久化、冻结快照与过期任务取消；桌面生命周期和 React 视图只通过该服务操作。两种分发方式复用应用身份与二进制，通过便携标记选择数据目录。

**Tech Stack:** Tauri 2、Rust、Tokio、Reqwest、Axum、serde-saphyr、React 19、Tailwind 4、Radix、Vite 8、Vitest／Testing Library、NSIS。

**Spec:** [设计说明](../specs/2026-10-16-subscription-server-upgrade-design.md)。执行前通读本计划与设计说明；本文件为待审阅计划，复选框不代表功能已经实现。

## Global Constraints

- 工作目录 `D:\code\ssr2clash`，分支 `codex/subscription-server-upgrade`，不重置或覆盖用户改动。
- 第一版只管理一个原订阅，一个稳定本地链接；SSR 输入、Clash／Mihomo YAML 输出。
- 默认监听 `127.0.0.1:25501`，上游单次获取超时 20 秒；最多 2 个同时执行的实时 HTTP 刷新，超限返回 503 与 Retry-After。
- 每个有效实时 GET 独立拉取原订阅；不使用 TTL、不合并请求；快照 GET 不访问上游或模板资源，响应字节保持一致。
- 最近成功结果与冻结快照分开保存；只回退当前配置版本的成功结果，持久化失败不能显示保存成功。
- 初次启动自启关闭、服务关闭；关闭窗口默认留托盘，明确退出停止服务；恢复用户之前的服务开启意图。
- 初始 profile 的 enable_udp 为 true，关键词为空、预设为 Builtin；初始主题跟随系统，获取方式为 Direct。
- 显示名 `SSR2Clash`，副标题“订阅转换与本地托管”；应用 identifier 保留 `com.redpandaex.ssr2clash`，exe／npm／Cargo 名仍为 `ssr2clash`。
- 银灰／白色与钴蓝 `#3869F5`，深色模式为石墨灰加蓝色；不用绿色／深青／薄荷色品牌主题。默认 1200×800、最低目标 1000×700。
- 便携版以 `portable.flag` 识别，数据在 exe 同目录 `data/`；安装版在用户应用数据目录；两者遵守同一单实例约束。
- 安装器必须有路径选择页，覆盖升级保留用户数据、令牌、端口与快照；用户未选清理时卸载保留数据。
- 依赖通过官方 CLI 调整并提交 lockfile；保留 Node 24、Tauri 2、React 19、Tailwind 4 的路线，不引入测试以外的第二套前端框架。
- 测试仅使用虚构订阅；日志不得输出完整原订阅 URL、凭证或 YAML；不自动发布 release 或创建标签。

## Review Focus

1. 返回 200 的 HTML 登录页／错误页不得替换有效订阅；由任务 4、6 的失败与回退测试覆盖。
2. 节点重名、空名或命名为 DIRECT／策略组名时，所有引用仍唯一有效；由任务 2 覆盖。
3. SSR 的 IPv6 地址含冒号、中文备注及不同 Base64 形式不能被主体切分破坏；由任务 1 覆盖。
4. 获取成功后磁盘保存失败，应保留旧状态与冻结快照，并向用户报告失败；由任务 5、6 覆盖。
5. 改配置、切模式或停止服务时仍有旧请求，旧结果不能覆盖新版本或冻结结果；由任务 6、7 覆盖。

## 文件与接口约定

Rust 沿用 `src-tauri/src/model.rs`、`ssr.rs` 的模块入口；新增模块各自放入目录，避免把全部逻辑堆进 `cmd.rs` 或 `lib.rs`。前端保留 Radix 基础组件，把业务组件放入 `src/components/workbench/` 与 `settings/`。

跨任务共享模型统一放在 `src-tauri/src/model/app.rs`，使用 Serde camelCase：

- `ProfileConfig { source_url: String, filter_keywords: Vec<String>, exclude_keywords: Vec<String>, enable_udp: bool, preset: PresetSource }`。
- `PresetSource`：`Builtin`、`LocalIni { path: PathBuf }`、`RemoteIni { url: String }`；JSON 使用 `kind` 标签，值为 `builtin`／`localIni`／`remoteIni`。
- `AppSettings { profile: ProfileConfig, port: u16, close_to_tray: bool, fetch_proxy: FetchProxy, theme: Theme }`；`FetchProxy` 为 Direct／System，Theme 为 Light／Dark／System。
- `OutputMode`：Live／Snapshot，JSON 为 `live`／`snapshot`。
- `GeneratedConfig { config_revision: u64, yaml: String, node_count: usize, group_count: usize, warnings: Vec<String>, generated_at: String }`；时间为 UTC RFC3339。
- `SavedState { schema_version: u32, revision: u64, subscription_token: String, settings: AppSettings, mode: OutputMode, service_enabled: bool, last_good: Option<GeneratedConfig>, frozen: Option<GeneratedConfig> }`；格式版本 1。
- `AppError { code: String, message: String }`；前端依赖稳定 code，message 必须脱敏。
- `CoreStatus { settings: AppSettings, revision: u64, mode: OutputMode, server_running: bool, subscription_url: Option<String>, active_result: Option<GeneratedConfig>, last_request_at: Option<String>, last_error: Option<String>, served_stale: bool }`。
- `DesktopStatus { run_mode: RunMode, data_dir: String, autostart_enabled: bool, app_version: String }`；RunMode 为 Portable／Installed，JSON 为小写。
- `AppView { core: CoreStatus, desktop: DesktopStatus }`；前端类型在 `src/lib/types.ts` 与 Rust 对应。
- `OutputDetails { yaml: String, nodes: Vec<ClashProxy>, groups: Vec<ClashProxyGroup>, decoded_preview: Option<String> }`；冻结结果从已保存 YAML 解读，不重新拉取。原订阅解码预览只保留在当前进程内。

模型建立顺序：任务 1 建立 AppError、ProfileConfig、AppSettings 及其枚举；任务 2 建立 GeneratedConfig／OutputDetails；任务 5 建立 SavedState／RunMode；任务 6 建立 CoreStatus；任务 8 建立 DesktopStatus／AppView。这样前置任务不会引用尚未实现的模型。

实施次序为 1→2→3→4→5→6→7→8→9→10→11，接口改变时同步更新其消费者和测试。每项完成后检查该项 diff 再提交；下述均是实施步骤，当前只编写计划。

## Task 1: 统一 SSR 解析与订阅内容识别

**Files:** Modify `src-tauri/src/ssr/parser.rs`、`src-tauri/src/model/ssr.rs`、`src-tauri/src/model.rs`、`src-tauri/src/lib.rs`；Create `src-tauri/src/model/app.rs`、`src-tauri/src/subscription/mod.rs`、`decode.rs`、`src-tauri/tests/subscription_decode.rs`、`tests/support/mod.rs`。

**Interfaces:** 继续提供 `parse_ssr_link(&str) -> anyhow::Result<SsrNode>`；新增 `parse_subscription(&str) -> Result<ParsedSubscription, AppError>`，其中 `ParsedSubscription { nodes: Vec<SsrNode>, invalid_count: usize, warnings: Vec<String>, decoded_preview: String }`。测试辅助 `ssr_link(server: &str, remarks: &str, group: &str, padding: bool) -> String` 构造 example.invalid／test-password 虚构节点。

- [x] **Step 1:** 用 fixture helper 写失败用例，替换现有空解析测试。明文与外层包装分别覆盖标准／URL-safe、有／无 padding；增加 IPv6、中文备注、坏端口与混入坏节点。断言：

```rust
assert_eq!(parse_ssr_link(&ssr_link("[2001:db8::1]", "香港 01", "测试", true))?.remarks, "香港 01");
assert_eq!(parse_subscription(&mixed_content)?.nodes.len(), 2);
assert_eq!(parse_subscription(&mixed_content)?.invalid_count, 1);
assert!(parse_subscription("<html>Sign in</html>").is_err());
```

- [x] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test subscription_decode`，确认在新 API 不存在或 padding／IPv6 用例处失败。
- [x] **Step 3:** 实现统一 Base64／UTF-8 处理；SSR 主体从右侧划分固定字段，保留 IPv6 主机；参数解码不把普通 `+` 误变为空格。移除测试空壳，错误报告只含节点序号与原因。
- [x] **Step 4:** 重新运行上述测试及 `cargo test --manifest-path src-tauri/Cargo.toml --lib`，确认非空解析用例、现有过滤测试通过。
- [x] **Step 5:** 提交本项解析、模型与测试文件，提交信息 `fix: normalize SSR subscription decoding`。

## Task 2: 完整 YAML、默认分组与离线规则

**Files:** Modify `src-tauri/src/model/clash.rs`、`model/app.rs`、`ssr/converter.rs`、`cmd.rs`、`Cargo.toml`／`Cargo.lock`；Create `src-tauri/src/preset/{mod.rs,model.rs,builtin.rs,rules.rs,validate.rs}`、`resources/presets/default/`、`tests/default_preset.rs`、`tests/fixtures/subconverter/`。

**Interfaces:** `ResolvedPreset { groups: Vec<GroupDefinition>, rules: Vec<String> }`；`load_builtin_preset() -> Result<ResolvedPreset, AppError>`；`build_clash_config(nodes: &[SsrNode], preset: &ResolvedPreset, enable_udp: bool) -> Result<ClashConfig, AppError>`；`validate_config(&ClashConfig) -> Result<(), AppError>`。

- [ ] **Step 1:** 固定一个可运行的 upstream subconverter 版本，以该版本 commit 固定默认配置和引用资源，并记录来源／许可证。用同一虚构 SSR 订阅生成对照输出；记录该版本实际支持 SSR 的 Clash 输出目标。测试比较组名称、顺序、成员身份和规则顺序；节点显示名可按 server／port／protocol 映射后比较，不把 emoji 显示差异当协议差异。
- [ ] **Step 2:** 写并运行 `cargo test --manifest-path src-tauri/Cargo.toml --test default_preset`，要求先失败于缺失分组／规则或错误字段。核心断言：

```rust
assert!(yaml.contains("proxy-groups:"));
assert!(yaml.contains("obfs-param:"));
assert!(!yaml.contains("obfs_param:"));
assert_eq!(config.rules.last().unwrap(), "MATCH,🐟 漏网之鱼");
assert_eq!(all_final_names.len(), unique_final_names.len());
```

- [ ] **Step 3:** 用 `cargo add serde-saphyr` 迁移 YAML、移除 serde_yaml；通过 Cargo 添加 fancy-regex 处理节点选择表达式。实现组模型、`udp` 输出、稳定唯一命名、保留声明顺序的规则生成和校验。DIRECT／REJECT 与所有组名参与节点名保留检查。`IP-CIDR,...,no-resolve` 的策略名插在 no-resolve 前。
- [ ] **Step 4:** 验证默认资源断网可用；测试空地区匹配仍有 DIRECT／有效组候选，空测速组、未知组、循环引用和重名组给出错误；重跑默认对照测试与 Rust 全部测试。
- [ ] **Step 5:** 提交转换器、资源、来源记录与测试，提交信息 `feat: generate complete Clash groups and routing rules`。

## Task 3: 可复用获取器与常用 INI 导入

**Files:** Create `src-tauri/src/subscription/http.rs`、`preset/ini.rs`、`preset/resolve.rs`、`tests/preset_import.rs`、`tests/support/upstream.rs`；Modify 模块入口与 `Cargo.toml`／lockfile。

**Interfaces:** `HttpFetcher::new(proxy: FetchProxy) -> Result<Self, AppError>`；`async fn get_text(&self, url: &str) -> Result<String, AppError>`；`async fn resolve_preset(source: &PresetSource, fetcher: &HttpFetcher) -> Result<ResolvedPreset, AppError>`。

- [ ] **Step 1:** 创建可计数的本地假上游 `TestUpstream`，提供 `url(path)`、`hits(path)`、`set_response(path,status,body)` 和可等待的阻塞响应。写 INI 测试：重复键与规则顺序、正则、`[]` 引用、`!!GROUP=`、相对导入、远程规则、循环／越界路径、失败资源。断言：

```rust
assert_eq!(preset.groups.len(), 2);
assert_eq!(preset.rules.last().unwrap(), "MATCH,Final");
assert_eq!(upstream.hits("/rules.list"), 1);
assert_eq!(error.code, "UnsupportedPresetOption");
```

- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test preset_import`，确认新 resolver 缺失或相关案例失败。
- [ ] **Step 3:** 通过 Cargo CLI 添加 Reqwest、Tokio、URL、fancy-regex 及实际需要的特性。实现 20 秒超时、无自动重试、直连／系统代理；原始响应限 8 MiB，导入总量限 16 MiB、深度限 8。INI 保留重复键、顺序与行号；支持 select／url-test／fallback／load-balance 的测速参数和常用规则列表，未知生效项报错。
- [ ] **Step 4:** 验证 HTTP 非成功状态、HTML 响应、超限、正则错误、文件越界、远程配置企图读取本地路径均失败；错误信息不含带 query 的原始 URL。重跑导入与默认预设测试。
- [ ] **Step 5:** 提交获取器、导入器与测试，提交信息 `feat: resolve local and remote subscription presets`。

## Task 4: 统一转换流水线与结构化输出

**Files:** Create `src-tauri/src/conversion.rs`、`tests/conversion_pipeline.rs`；Modify `src-tauri/src/lib.rs`、`cmd.rs`、`Cargo.toml`、`Cargo.lock`。

**Interfaces:** `#[async_trait] trait Converter: Send + Sync { async fn generate(&self, profile: ProfileConfig, revision: u64) -> Result<ConversionResult, AppError>; }`；`ConversionEngine` 实现该接口。`ConversionResult { output: GeneratedConfig, details: OutputDetails }`；engine 依次获取、解析、过滤、解析预设、生成、校验、序列化。

- [ ] **Step 1:** 对本地假上游写完整流水线失败测试。两节点、十组的样本必须断言真实节点数为 2、组数为 10；故意返回 200 HTML、全坏节点、过滤为空和缺远程规则时必须失败。
- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test conversion_pipeline`，确认在接口／计数／无效内容断言处失败。
- [ ] **Step 3:** 实现 `ConversionEngine` 与共享类型，通过 Cargo 添加 async-trait 与时间处理依赖。生成时间用 UTC RFC3339，按最终结构计数。旧手动命令临时调用该公共核心或同一纯转换函数，删除重复前端统计所依赖的后端歧义。
- [ ] **Step 4:** 用 `serde_saphyr::from_str::<ClashConfig>(&result.output.yaml)` 回读并校验，断言 config_revision 原样为传入值；两次 generate 各增加一次上游 hits；所有错误做脱敏断言。
- [ ] **Step 5:** 提交核心与测试，提交信息 `feat: centralize subscription conversion in Rust`。

## Task 5: 便携／安装数据目录与原子持久化

**Files:** Create `src-tauri/src/storage/{mod.rs,paths.rs,file.rs}`、`tests/storage_state.rs`；Modify `src-tauri/src/model/app.rs`、`lib.rs`、`Cargo.toml`、`Cargo.lock`。

**Interfaces:** `detect_data_paths(exe: &Path, app_data: &Path) -> Result<DataPaths, AppError>`，`DataPaths { run_mode: RunMode, root: PathBuf }`；`#[async_trait] trait Storage: Send + Sync { async fn load(&self) -> Result<Option<SavedState>, AppError>; async fn save(&self, state: &SavedState) -> Result<(), AppError>; }`；`FileStore` 实现 Storage。

- [ ] **Step 1:** 用 tempfile 的独立目录写失败测试，覆盖 portable.flag、当前工作目录变化、没有标记、覆盖已有状态与未知 schema。注入测试用存储故障，断言失败后原 state 文件完整可读、原冻结 YAML 未变。
- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test storage_state`，先确认缺少目录识别或原子覆盖实现。
- [ ] **Step 3:** 按 exe 的父目录识别标记，默认 Installed 使用传入 app_data。将完整 SavedState 作为一次提交原子写入；临时文件与目标同目录。用在 Windows 支持替换的机制，保存失败保留旧文件并清理本次临时文件。首次创建随机令牌并持久化。
- [ ] **Step 4:** Windows 上实际验证覆盖、重新加载、中文／空格目录；测试生成令牌不会因重启改变，Portable 不静默退到 AppData，坏文件不会被默认空配置覆盖。
- [ ] **Step 5:** 提交存储与测试，提交信息 `feat: persist portable and installed application state`。

## Task 6: 实时／快照状态机与过期任务保护

**Files:** Create `src-tauri/src/service/{mod.rs,refresh.rs,snapshot.rs}`、`tests/service_state.rs`、`tests/support/store.rs`；Modify `src-tauri/src/model/app.rs`、`lib.rs`。

**Interfaces:** `AppService::open(store: Arc<dyn Storage>, converter: Arc<dyn Converter>) -> Result<Self, AppError>` 为 async 构造；async 方法 `status() -> CoreStatus`、`save_settings(AppSettings)`、`set_snapshot(bool)`、`refresh() -> Result<GeneratedConfig, AppError>`、`serve_subscription() -> Result<ServedYaml, AppError>`、`set_service_enabled(bool)`、`cancel_refreshes()`。状态修改方法返回 `Result<CoreStatus, AppError>`；取消方法返回 `()`。`ServedYaml { yaml: String, stale: bool }`。

- [ ] **Step 1:** 用可阻塞 Converter 与故障 Storage 写失败测试：

```rust
assert_eq!(converter.calls(), 2); // 两次 live 请求各自刷新
assert_eq!(frozen_response_1.yaml, frozen_response_2.yaml);
assert_eq!(converter.calls(), calls_before_snapshot_requests);
assert_eq!(error.code, "NoSnapshotAvailable");
assert_eq!(service.status().await.active_result, old_result); // 保存失败后不发布新结果
```

- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test service_state`，确认接口或每一类语义先失败。
- [ ] **Step 3:** 实现持久化状态的串行转换：先保存候选状态，成功后更新内存；网络转换不持状态锁。配置 revision 仅随影响输出的字段递增；源改变使旧回退无效。frozen 是 last_good 的独立拷贝，冻结后输出配置不可编辑；主题等非输出字段仍可编辑。
- [ ] **Step 4:** 用 Notify／oneshot 屏障而非 sleep 测试旧请求完成晚于改源、切快照、停止服务；通过 generation epoch 与取消通知丢弃旧提交。测试同版本失败回退与无结果错误、服务开启意图跨重启，以及冻结时 refresh 返回 `SnapshotFrozen`。
- [ ] **Step 5:** 提交状态服务与测试，提交信息 `feat: add durable live and frozen subscription modes`。

## Task 7: Axum 订阅端点与可控服务生命周期

**Files:** Create `src-tauri/src/server/{mod.rs,handlers.rs}`、`tests/local_server.rs`；Modify 依赖和模块入口。

**Interfaces:** `ServerController::new(service: Arc<AppService>) -> Self`；async `start(port: u16) -> Result<(), AppError>`、`stop() -> Result<(), AppError>`；绑定 127.0.0.1。start 绑定成功且开启意图保存成功后才报告运行；若保存失败，释放新监听。stop 取消 HTTP 刷新，有界退出并保存关闭意图。

- [ ] **Step 1:** 基于真实临时 TCP 端口和可计数假上游写失败测试：GET／HEAD／health、错误令牌、快照重复请求、回退、无缓存 502、端口占用、限流和停止释放。测试 start(0) 仅供测试自动分配端口，生产 settings 仍默认 25501。
- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test local_server`，确认新端点未实现。
- [ ] **Step 3:** 添加 Axum，路由 `/sub/{token}.yaml` 和 `/health`；HEAD 单独读取当前结果的头信息，不调用刷新逻辑。设置 YAML Content-Type、Content-Disposition、no-store；失败返回非 200 文本，错误令牌 404。实时入口用 2 个 permit，超限 503、Retry-After: 1。增加服务内部的 `mark_server_running(bool)` 方法，只由 ServerController 更新实际监听状态，不能用持久化开启意图冒充正在运行。
- [ ] **Step 4:** 断言两次实时 GET hits 为 2，快照 GET hits 不增加且字节相同，HEAD hits 不增加；同时占满两个慢请求时第三个返回 503。启动／停止可重复，运行时改端口返回 `ServerMustStop`，停止后端口可重新绑定；没有配置的服务启动返回 `InvalidSourceUrl`。
- [ ] **Step 5:** 提交 HTTP 服务与测试，提交信息 `feat: serve local Clash subscriptions over HTTP`。

## Task 8: Tauri 命令、托盘、单实例和自启

**Files:** Create `src-tauri/src/desktop/{mod.rs,lifecycle.rs,tray.rs,autostart.rs}`；Modify `lib.rs`、`cmd.rs`、`capabilities/default.json`、`Cargo.toml`、`tauri.conf.json`；Create `tests/desktop_policy.rs`。

**Interfaces:** 命令 `get_state() -> AppView`、`get_output_details() -> OutputDetails`、`save_settings(settings: AppSettings) -> AppView`、`refresh_subscription() -> AppView`、`set_snapshot_enabled(enabled: bool) -> AppView`、`start_server() -> AppView`、`stop_server() -> AppView`、`set_autostart_enabled(enabled: bool) -> AppView` 均为 async `Result<_, AppError>`。事件 `app-state-changed` 仅通知重新读取状态，不发送原订阅内容。

- [ ] **Step 1:** 用桌面策略纯函数测试 `--autostart` 隐藏启动、关闭窗口留托盘、明确退出停止、手动停止后的启动恢复行为；断言 autostart_enabled 取操作系统实际注册结果，注册失败不显示为开启。
- [ ] **Step 2:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test desktop_policy`，确认策略／接口尚未实现。
- [ ] **Step 3:** 先解析实际 pnpm 入口，固定一份可工作的 pnpm 10 稳定版本并写入 packageManager；不能重复使用已知会触发隐式重装和 SQLite 错误的入口。随后通过 `pnpm tauri add autostart` 与 `pnpm tauri add single-instance` 安装官方插件。初始化时先确定数据目录，单实例插件保证整个应用身份只运行一个进程。后台服务由 Rust 管理，不依赖 WebView 持续运行。托盘菜单接同一服务动作。
- [ ] **Step 4:** 命令注册、错误脱敏、权限最小化并通过 cargo check；在 Windows 实际验证关窗后 HTTP 仍可访问、第二次打开恢复窗口、退出释放端口、自启注册与取消。便携路径变动时展示真实注册状态并可重新注册，不擅自改系统其他启动项。
- [ ] **Step 5:** 提交桌面集成，提交信息 `feat: integrate tray lifecycle and optional autostart`。

## Task 9: 现代桌面工作台与依赖升级

**Files:** Modify `package.json`、lockfile、Vite／TypeScript／Biome 配置、`src/App.tsx`、`src/styles/index.css`、`src/lib/types.ts`、`src/lib/api/subscription.ts`；Create `src/components/pages/WorkbenchPage.tsx`、`src/components/workbench/{SubscriptionForm,ServiceInspector,OutputWorkspace,RuntimeLog}.tsx`、`src/components/settings/SettingsDialog.tsx`、`src/hooks/useAppState.ts`、`src/test/setup.ts`、相关 `.test.tsx`。

**Interfaces:** `subscriptionApi` 导出 camelCase 函数 getState、getOutputDetails、saveSettings、refreshSubscription、setSnapshotEnabled、startServer、stopServer、setAutostartEnabled，对应任务 8 命令；`useAppState()` 提供 `{ view, output, pending, error, actions }`。UI 完全使用结构化输出，不扫描 YAML 统计节点。

- [ ] **Step 1:** 复用任务 8 固定的 pnpm 入口，CI 使用同一版本。通过 CLI 升级 React 19、Vite 8、匹配的 React 插件、Tailwind 4、TypeScript／Biome 及 Tauri 2 JS 依赖；Node 类型对齐 Node 24。为实际 UI 行为添加 Vitest／Testing Library，`scripts.test` 设置为 `vitest run`，而非对配置版本写镜像测试。
- [ ] **Step 2:** 写失败 UI 测试，使用类型化 API mock：空订阅禁用更新，快照无结果时不能开启，刷新失败仍显示旧结果与持续错误，冻结后输出编辑不可用，停止后状态更新，主题不改变服务配置。断言示例：

```tsx
expect(screen.getByRole('button', { name: '立即更新' })).toBeDisabled();
expect(screen.getByText('上次更新失败，正在使用之前的配置')).toBeVisible();
expect(screen.getByText('2 个节点')).toBeVisible(); // 不把 10 个策略组算成节点
```

- [ ] **Step 3:** 运行 `pnpm test`，确认功能断言先失败；按页面概念稿实现银灰／钴蓝工作台，窄导航、顶部输入行、宽结果区与窄服务检查区。默认预设文案必须是“内置默认分流”，不是“不使用（直连）”。实际尺寸与文字以设计说明的可读性为准，不复制生成图的文字误差。
- [ ] **Step 4:** 运行 `pnpm test`、`pnpm build` 与相关 lint；浏览器检查 1200×800、1000×700，浅／深色与空态／加载／失败／冻结态。键盘可达、中文不截断、长 YAML 和地址不撑宽；运行记录默认折叠且最多 100 条脱敏记录。保存失败时表单保留用户输入，不能自动覆盖成旧值。
- [ ] **Step 5:** 在真实 Tauri 窗口验证 IPC、复制、导出、主题与后台事件；移除已经替代的旧页面和无引用业务组件后提交，提交信息 `feat: rebuild the subscription desktop workbench`。

## Task 10: 原创图标与统一应用身份

**Files:** Create `assets/branding/icon.svg`、`assets/branding/tray-light.svg`、`tray-dark.svg`；Modify `src-tauri/icons/`、`public/`、`index.html`、`tauri.conf.json`、UI 品牌入口与桌面托盘资源。

**Interfaces:** 从设计目录中的两端连接 SVG 确定源图，采用石墨／白／蓝配色；产品显示名 SSR2Clash，副标题“订阅转换与本地托管”。只替换展示资源，保留既有应用标识与文件名。

- [ ] **Step 1:** 查看 `docs/superpowers/assets/ssr2clash-icon-concept.svg` 与 PNG，按最终审阅方向生成正式 SVG；托盘保持同一轮廓的单色透明图形。此项不为颜色值或静态 SVG 路径增加镜像单元测试。
- [ ] **Step 2:** 运行 `pnpm tauri icon assets/branding/icon.svg --output src-tauri/icons`，使用官方 CLI 生成平台图标；生成供托盘直接读取的浅／深主题小尺寸 PNG。
- [ ] **Step 3:** 检查 PNG 为 RGBA 正方形、ICO 包含 16／24／32／48／64／256 尺寸，肉眼查看 16／24／32 像素下识别度；窗口、任务栏、托盘和 UI 都加载新图标。
- [ ] **Step 4:** 重跑前端打包和 Rust 检查，并确认安装器、快捷方式的图标来源已指向生成资源，不保留默认 Tauri 标志。实际安装后的壳图标由任务 11 验证。
- [ ] **Step 5:** 提交正式源图与生成资源，提交信息 `feat: refresh SSR2Clash application identity`。

## Task 11: 双版本打包、可选路径与交付验证

**Files:** Create `scripts/package-windows.ps1`、`src-tauri/installer/hooks.nsh`、`.github/workflows/ci.yml`、`docs/windows-verification.md`；Modify `.github/workflows/release.yml`、`README.md`、`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、相关 lockfile；打包临时目录置于已忽略的 target／dist 下。

**Interfaces:** `package-windows.ps1` 从同一提交生成 `ssr2clash_<version>_windows_x64_portable.zip` 和 `ssr2clash_<version>_windows_x64_setup.exe`；便携 ZIP 有 exe、portable.flag、说明，没有预置数据或令牌。三个应用版本统一为本次迭代的 `1.1.0`。

- [ ] **Step 1:** 为打包脚本写可直接运行的产物检查：两种包存在且非空，ZIP 条目符合清单、无 data／订阅／令牌；exe 产品版本与安装器版本一致。此项验证真实产物，不用只检查配置文件字段的测试替代安装验证。
- [ ] **Step 2:** 启用 NSIS bundling，当前用户安装、正常交互有目录选择页；升级读取已安装目录。按实际安装路径创建快捷方式／卸载项，卸载只清理属于该路径的自启注册，用户未选清理时保留 AppData。保留原始 NSIS 模板，优先使用官方配置和必要的 hooks。
- [ ] **Step 3:** 通过官方构建命令实际生成 exe 与 NSIS 包，再由脚本组装便携 ZIP。CI 对分支／PR 执行前后端检查并归档两个构建产物；已有 tag 发布流程上传两种包、固定 pnpm，检查失败时不发布半套产物。
- [ ] **Step 4:** 在隔离 Windows 环境验证自定义路径、D 盘路径、中文／空格路径、不可写目录；安装→使用虚构订阅生成快照→覆盖升级→重启应用，确认端口、令牌和快照保留。分别验证默认保留数据与明确清理数据的卸载行为，以及便携版更换工作目录／整体移动／覆盖升级。检查任务栏、开始菜单、快捷方式、自启的真实路径与图标。
- [ ] **Step 5:** 运行最终 `pnpm test`、`pnpm build`、相关 lint，以及带 `--manifest-path src-tauri/Cargo.toml` 的 cargo fmt／test／check 命令，记录命令和结果。将实际运行的检查、包路径、已验证和待物理验证事项写入 windows-verification.md；用户真实订阅／Clash Verge 导入与实际重启登录验证不由假上游或构建成功替代。提交信息 `build: ship portable and installable Windows packages`。

## 自检与执行交接

- 设计说明各章节均映射到上面的任务；Review Focus 的五类输入均有指定测试归属。
- 不把图标预览或 AI 生成页面图当成产品截图；界面稿只约束视觉方向，真实行为由 UI 与集成测试验证。
- 运行代码、安装产品依赖和更换正式打包资源从计划审阅通过后开始。
- 推荐 Native：由当前聊天连续实施，逐项完成测试与提交，最后独立复查整分支。转换、状态机、HTTP、桌面与 UI 依赖同一套类型，连续实施有利于保持接口一致。
- 用户审阅计划后确认执行方式；如果改选 Subagent-driven，再使用相应技能进行任务分工。
