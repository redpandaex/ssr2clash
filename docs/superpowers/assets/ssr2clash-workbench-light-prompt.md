# 主界面概念稿生成记录

用途：讨论界面方向，不是已经实现的应用截图。

工具：内置 image_gen，生成新图；用户要求移除绿色主题，改为现代桌面工具风。

最终导出：`ssr2clash-workbench-light-concept.png`。另通过同一内置工具编辑默认规则下拉框文字，将生成图中的“不使用（直连）”纠正为“内置默认分流”，保持其他视觉元素不变。

编辑提示词：

```text
Use case: precise-object-edit of the supplied desktop UI concept. Change ONLY the selected text in the preset dropdown under the label 默认规则: replace 不使用（直连） with the exact simplified Chinese text 内置默认分流. This is important because the app must use bundled routing rules by default. Keep ALL other UI content, iconography, sizes, composition, silver-gray/white/cobalt palette, typography, border treatments, and native window frame exactly unchanged. No green or teal colors. Do not add any elements.
```

原始生成提示词：

```text
Use case: ui-mockup.
Create ONE high-fidelity, production-plausible Windows desktop application screen for SSR2Clash, a Chinese single-subscription converter and local subscription server. This is a fresh design direction after the user rejected a green/teal theme and asked for a more modern interface.
Style: a refined modern desktop productivity app, silver-gray and white surfaces, strong ink typography, cobalt blue #3869F5 as the ONLY primary accent. Precise spacing, crisp borders, small 8px radii, subtle hierarchy, generous usable workspace. NO GREEN, NO TEAL, NO MINT UI OR BRAND COLORS. No gradients, no glass panels, no giant marketing hero, no decorative dashboards, no fake charts, no cards nested inside cards.
Framing: a single complete desktop application screenshot in a landscape 16:10 ratio, approximately 1600 by 1000. The app fills the image. Thin Windows-style titlebar with minimize/maximize/close at the right, not macOS traffic lights. No laptop/device mockup or surrounding room.
Information architecture: a narrow quiet pale-gray sidebar with a small original cobalt/white icon of two endpoints connected by a flowing line, the exact brand SSR2Clash, and three modest navigation items 工作台, 输出预览, 设置. Workbench active, restrained cobalt highlight. Bottom sidebar has theme toggle and 开机自启.
Main workspace: top page heading 工作台 with a smaller subtitle 订阅转换与本地托管. Right aligned thin blue outlined service status 运行中 and a quiet 停止服务 button.
Below, a single clean full-width subscription input row labeled 原订阅地址 with placeholder https://example.invalid/subscribe and a compact cobalt 立即更新 button. Below this row, small unobtrusive 包含关键词 and 排除关键词 controls, and a quiet 默认规则 preset selector. Do not crowd this area with large cards.
Below, an asymmetric workspace: a broad node/output area and a narrower right-hand service inspector separated by whitespace and a fine divider. Left area has tabs 节点, 策略组, YAML, with 节点 active, a small count 80 个节点 · 10 个策略组, and a clear, roomy table with columns 节点, 协议, 地址, 端口. Show four or five realistic synthetic rows named 香港 01, 香港 02, 日本 01, 美国 01, 新加坡 01; protocol SSR; addresses such as hk01.example.invalid; port 8388. No green dots, flags, or status decoration.
Right inspector heading 本地订阅, a small blue chain-link icon, a readable copyable local address http://127.0.0.1:25501/sub/demo.yaml and 复制链接 action. A mode segmented control 实时 and 快照, with 实时 selected in cobalt. Caption 每次请求重新拉取原订阅. A subtle divider, then 更新于 14:32 and 最近请求 14:35. A quiet 导出 YAML button near the bottom.
A low-height collapsible 运行记录 strip at the bottom. Default view fits a small laptop without scrolling. Text must be legible, aligned, spelled correctly in simplified Chinese; avoid unnecessary text. Brand typography is medium weight and compact, not huge.
The composition should feel intentionally designed and clean enough to implement faithfully with React, Radix, and Tailwind. Show real usable controls and deliberate density.
```
