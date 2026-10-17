# Windows 安装器

Tauri 使用 NSIS 将应用和资源打包为 Windows 安装版 `Setup.exe`。`tauri.conf.json` 中的 `bundle.windows.nsis` 指定了本目录的模板和钩子。

| 文件 | 用途 |
| --- | --- |
| `installer.nsi` | 安装/卸载主模板，包含安装路径选择、覆盖升级、快捷方式和数据清理页面。 |
| `hooks.nsh` | 安装目录可写检查、便携目录保护，以及按程序路径判断自启项归属和迁移。 |
| `upstream.json` | 官方模板的来源、版本、原始 SHA-256 和本地修改说明。 |

模板基于 Tauri CLI 2.12.3。主模板仅将无条件删除同名自启项的逻辑替换为钩子中的路径检查，避免卸载安装版时删除另一份便携版的自启项。

升级 Tauri CLI 时，对照 `upstream.json` 中的官方模板检查差异，并保留上述自启归属逻辑。当前安装版构建依赖这些文件。
