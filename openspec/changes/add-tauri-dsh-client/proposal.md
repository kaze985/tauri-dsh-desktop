## Why

DSH (DeepSeek Harness) 通过 `dsh web` 在本地启动 Web 服务，日常使用必须手动保持一个终端和一个浏览器窗口，启动步骤繁琐且不优雅。本项目用一个 Tauri 2 Windows 桌面客户端把"启动 → 使用 → 退出"收敛为双击图标一件事，同时作为学习 Tauri 框架的练手项目。

## What Changes

- 新建 Tauri 2 桌面应用（Rust 后端 + 极简 WebView 壳层），Windows 平台。
- 客户端负责 dsh web 服务的完整生命周期：启动前端口预检、就绪轮询、进程树整体终止、崩溃与超时的错误呈现。
- 壳层前端提供三个本地状态页：loading、错误页（含重试/退出）、就绪后导航到 `http://127.0.0.1:3080`（不内嵌任何 DSH UI，插件生态免费继承）。
- 系统托盘 + 关闭询问对话框（退出 / 最小化到托盘，支持"不再询问"记忆）。
- dsh 通过全局 npm 安装（`npm i -g @deepseek-ai/dsh`），客户端保证安装存在；启动时后台比对 npm registry 与当前安装版本，提示更新，用户确认后执行升级并重启服务。
- 单实例运行：二次启动聚焦已有窗口，防止多开争抢 `~/.dsh` 配置与 3080 端口。

## Capabilities

### New Capabilities

- `server-lifecycle`: dsh web 服务进程的启动、就绪检测、进程树终止、崩溃与端口冲突处理。
- `window-shell`: 壳层窗口的状态机（loading / 错误页 / 就绪导航）与启动序列 UI。
- `tray-and-close`: 托盘驻留、关闭对话框语义与用户选择记忆。
- `version-management`: 全局安装的 dsh 版本检测、registry 比对、用户确认式升级。
- `single-instance`: 应用级单实例守卫与二次启动的窗口聚焦行为。

### Modified Capabilities

<!-- 无：新仓库，openspec/specs 为空 -->

## Impact

- **新增**：整个 Tauri 项目（Rust crate、前端壳层、Tauri 配置与图标）。
- **依赖**：Tauri 2（tauri / tauri-plugin-single-instance / tray / dialog）、系统 npm（Node 18+，本机 npm 11 已满足）、Windows WebView2。
- **外部系统**：npm registry（`npm view @deepseek-ai/dsh version`）、全局 npm 安装的 `@deepseek-ai/dsh`、本机 `dsh web` 服务（端口 3080）。
- **风险面**：对 `dsh web` 的子进程管理依赖 Windows 进程树语义（`taskkill /T /F`）；升级操作会重启 dsh 服务，中断进行中的会话。
