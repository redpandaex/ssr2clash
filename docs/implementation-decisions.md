# SSR2Clash 1.1.0 实现决策与复查

基线：main 的 71caca2b0b87457763c70411b8c131b6365388c6；迭代分支：codex/subscription-server-upgrade。

11 项计划任务已完成。一次独立整分支复查提出 3 项重要问题，均经先失败、后通过的回归测试修复；未再次扩展审查范围。最终 Rust 测试共 78 项，前端行为测试共 9 项。真实设备和完整安装向导的手动验收边界见 windows-verification.md。

## Rulings

以下保留执行过程中的全部裁定与其代价：
- Ruling: use the existing checkout on the user-requested codex/subscription-server-upgrade branch, without another worktree — user explicitly requested this branch in the shared project — cost if wrong: a separate checkout can be created later without changing commits.

- Task 2: Ruling: bundle normalized rules emitted by official subconverter v0.9.0 (9797 rules), with original group/ruleset indexes and license — preserves exact offline rule order while excluding source-only Surge directives as upstream does — cost if wrong: regenerate bundled defaults from the recorded release. Arbitrary INI import remains separately validated in Task 3.

- Task 3: Ruling: accept plain YAML clash_rule_base and preserve its non-node fields so enable_rule_generator/overwrite_original_rules have real semantics — extend ResolvedPreset with base map and ClashConfig with flatten extra — cost if wrong: a small schema/model adjustment; scripts and template languages remain unsupported. Disabled generation skips unnecessary rule fetches.

- Task 4: Ruling: Converter.generate takes fetch_proxy as a per-call third argument — the approved AppSettings stores proxy mode outside ProfileConfig, so a fixed client would ignore later UI changes — cost if wrong: adjust the internal trait consumers. GeneratedConfig gains invalid_count to satisfy the spec numeric invalid-node metadata requirement.

- Task 7: Ruling: enforce the two-generation quota in AppService rather than duplicate route permits; publish actual listening port through mark_server_running(Option<u16>) — disconnected callers must not release quota while owned work continues, and ephemeral test ports must report correctly — cost if wrong: small internal controller/service API change.

- Task 8: Ruling: use a current-user Windows registry adapter for quoted autostart commands and current-executable ownership checks, retaining the official Tauri plugin for other desktop platforms — inspected auto-launch 0.6.0 Windows source writes an unquoted executable path and considers any same-name registration enabled; this fails space paths and moved portable folders — cost if wrong: replace the adapter when upstream supports these semantics. The adapter does not touch machine-wide entries or another path on disable.

- Task 8: Ruling: add ServerController.shutdown() distinct from manual stop() — the spec says normal exit releases the listener while subsequent startup restores saved intent; only a manual stop clears intent — cost if wrong: change one lifecycle policy. Added a real TCP regression RED missing shutdown then GREEN.

- Task 9: Ruling: honor the users manual native-preview preference and do not open more desktop windows — prior native smoke verified IPC/lifecycle, current UI behavior/copy/export intent/theme/focus are checked headlessly with synthetic API fixtures; new UI clipboard/file dialog/OS chrome integration remains explicitly manual — cost if wrong: an integration issue may require a follow-up after manual preview.

- Task 9: Ruling: Tauri beforeDev/beforeBuild invoke corepack pnpm with the pinned packageManager version — avoids the broken Codex fallback pnpm wrapper in nested commands while matching CI — cost if wrong: development hosts must provide Corepack (verified locally with Node24).

- Task 11: Ruling: vendor the pinned official NSIS2.12.3 template with one deletion-block replacement, retaining the rest of its pages and data behavior — default NSIS unconditionally deletes a same-name Run entry and can delete another portable copy startup; hook-only restoration would introduce a deletion window — cost if wrong: update and re-audit the small template patch when the CLI template changes. Source SHA and licenses are recorded.

- Task 11: Ruling: do not execute the full installer GUI/installed application upgrade/uninstall on this active desktop after the user requested manual preview — built and inspected actual artifacts; tested the same ownership/migration/path macros using a silent NSIS harness under a random dedicated registry root; full installation and device acceptance are explicitly manual in windows-verification.md — cost if wrong: an installer integration issue may remain until isolated manual acceptance.

- Final: minor (deferred): second-instance activation does not show the actual running executable path when another distribution is already open; runtime mode/data directory remain visible in settings. New-path prompt is deferred.

- Final: Ruling: native UI clipboard/file dialog/theme/tray appearance remains manual — respects the user manual-preview request; headless checks plus previously verified native IPC do not replace these operations — cost if wrong: a native UI integration fix may be required after preview.

- Final: Ruling: full installed upgrade/uninstall and portable folder move remain manual — silent hook cases verify shared macro logic and artifact inspection, not full device flows — cost if wrong: installer integration issues may appear during isolated acceptance.

- Final: Ruling: real SSR/Clash Verge and actual login reboot remain user-device acceptance — synthetic subscription and simulated autostart are explicitly distinguished — cost if wrong: client-specific or login-environment behavior may need follow-up.

- Final: Ruling: no claims for platforms other than Windows x64 — approved distribution and verification scope is Windows x64 — cost if wrong: other platforms need their own build and runtime validation.

- Final: Ruling: merge ordinary base/generated rules before one terminal MATCH and reject differing final targets unless overwrite_original_rules is enabled — prevents silently unreachable rules or silently changing the chosen fallback — cost if wrong: users must explicitly reconcile the fallback in their preset.

- Final: Ruling: cap groups512, selectors8192, group-definition bytes1MiB, member references250000, member-name bytes16MiB, match checks10000000, rules100000 and rule bytes16MiB; reject duplicate definitions at import time — bounds expansion before cloning/serialization and validates persisted output too — cost if wrong: unusually large presets need smaller node selections or a deliberately reviewed limit adjustment.
