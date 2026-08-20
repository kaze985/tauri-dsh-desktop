# DSH Desktop

Windows 桌面客户端，用 Tauri 2 薄壳封装 [DeepSeek Harness (DSH)](https://www.npmjs.com/package/@deepseek-ai/dsh)：双击图标即启动 `dsh web`，关窗可选托盘驻留，退出时杀干净整个进程树。

## 设计

- **薄壳路线**：WebView 就绪后导航到 `http://127.0.0.1:3080`，不内嵌任何 DSH UI，SSH/任务看板等插件生态免费继承。
- **全局 npm 安装 dsh**（不走 npx）：npx 按 spec 分哈希缓存，"裸包名"会永停旧版、手动更新喂不到客户端；全局安装使"当前版本"成为唯一事实。
- 生命周期：端口预检 → `dsh web --host 127.0.0.1 --port 3080`（CREATE_NO_WINDOW）→ TCP 轮询就绪（300ms × 30s）→ 导航；退出统一 `taskkill /PID <pid> /T /F` 杀进程树。
- 托盘菜单：显示主窗口 / 检查更新 / 重置关闭行为 / 退出。
- 更新检查：后台比对 `dsh --version` 与 `npm view @deepseek-ai/dsh version`，用户确认后 `npm i -g` 升级并重启服务；断网静默跳过。
- 单实例：tauri-plugin-single-instance，二次启动聚焦已有窗口。

## 开发

前置：Node 18+、npm、Rust stable-msvc、Visual Studio Build Tools（C++ 工作负载）、Windows WebView2。

```bash
npm install
npm run tauri dev      # 开发模式（前端 Vite 在 1420）
```

## 打包

本机网络直连 GitHub release 资产会被重置（WiX/NSIS 下载超时），打包时走镜像：

```bash
TAURI_BUNDLER_TOOLS_GITHUB_MIRROR=https://gh-proxy.com/https://github.com npm run tauri build
```

产物：`src-tauri/target/release/bundle/` 下的 NSIS exe 与 MSI。

## 测试

```bash
cd src-tauri && cargo test   # 5 个测试：版本比较 ×3、端口预检、进程树杀死
```

## 验收（DoD）

1. 双击安装 → 启动「DSH Desktop」→ loading → 进入 DSH UI（需 3080 空闲）。
2. 点 X → 询问【退出程序 / 最小化到托盘】；勾选「不再询问」后行为被记忆；托盘菜单「重置关闭行为」可恢复询问。
3. 退出后 `tasklist` 无残留 node/dsh，3080 已释放。
4. 二次启动时聚焦已有窗口（单实例）。
5. 托盘「检查更新」：最新版提示"已是最新"；断网时静默跳过不影响启动。

## 变更管理

OpenSpec（spec-driven）：提案与任务见 `openspec/changes/add-tauri-dsh-client/`。
