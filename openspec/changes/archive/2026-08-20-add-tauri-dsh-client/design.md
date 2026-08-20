## Context

动机见 proposal.md。技术约束：

- 本机为 Windows 11 + Git Bash，已装 Node（npm 11.12.1）、WebView2 由系统提供。
- DSH 以 npm 包 `@deepseek-ai/dsh` 分发（当前 rc.8），`dsh web` 默认绑定 `127.0.0.1:3080`，支持 `--port` 显式指定；服务就绪前端口不通。
- 实测（本机 npm 11）：npx 按 spec 分哈希缓存目录，"裸包名"与 "@latest" 是两条互不相交的缓存线（裸包名已永久停在 rc.6，@latest 目录为 rc.7，registry 最新 rc.8），手动 `@latest` 更新喂不到裸包名。**这是决策 D（全局安装）的直接依据。**
- 工作目录为空仓库，OpenSpec 已初始化（spec-driven）。

## Goals / Non-Goals

**Goals:**

- 以最少的壳层代码实现"双击 → loading → DSH UI → 托盘/退出"的完整闭环（见 specs/window-shell、server-lifecycle）。
- 覆盖 Tauri 核心 Rust 侧知识：子进程管理、事件、托盘、窗口控制、单实例插件、Windows 进程树语义。

**Non-Goals:**

- 不内嵌 DSH UI、不碰插件生态（导航到 3080 后全部由 DSH 自身提供）。
- 不做壳自身更新、不做多机器分发、不做 Win7/8 兼容。
- 不用 IPC 命令体系构建业务功能（壳层与前端仅通过事件单向通信）。

## Decisions

### D1: dsh 安装与启动走全局 npm，彻底绕开 npx

客户端保证 `npm i -g @deepseek-ai/dsh` 已装，spawn `dsh web --port 3080`。

- **理由**：全局安装使"当前安装版本"成为唯一真实概念（`dsh --version` 直接读），手动 `npm i -g …@latest` 更新立即对客户端生效；离线可启动；无 npx 哈希缓存分叉。
- **替代方案**：a) 裸包名 npx —— 被实测否决（缓存永停旧版）；b) 配置锁版本 npx —— 等价于把锁挪到配置文件，多一层间接。均不采用。

### D2: spawn 用 Rust `std::process::Command`，`CREATE_NO_WINDOW` 隐藏控制台

`dsh` 是 npm 全局安装的 .cmd 包装；`std::process::Command` 用 CreateProcess 直接解析、不查 PATHEXT，因此统一经 `cmd /C` 调用（`Command::new("cmd").args(["/C", "dsh", ...])`）。
补充（实现期发现）：spawn 追加 `--no-open` —— dsh-web-app 的 `openBrowser` 默认 true，`dsh web` 启动时会拉起默认浏览器，与客户端 WebView 形成双窗口；`--no-open` 是官方关闭开关（`dsh --profile web --help` 可见）。

- **理由**：零额外依赖；`creation_flags(0x08000000)` 防止弹出黑色控制台窗口。
- **替代方案**：tauri-plugin-shell —— 为单个长驻子进程引入插件不值得，且其 sidecar 机制面向打包二进制，不适配 npm 全局 CLI。

### D3: 就绪检测 = TCP 轮询，超时 30s

每 300ms 尝试 `TcpStream::connect(127.0.0.1:3080)`，成功即就绪；失败达 30s 转错误态。

- **理由**：跨平台、无输出解析、对 harness 内部实现零假设。
- **替代方案**：解析 stdout 日志 —— 依赖 DSH 日志格式，rc 期随时会变。

### D4: 退出杀整棵进程树：`taskkill /PID <pid> /T /F`

- **理由**：`child.kill()` 只杀直接子进程；cmd → node → dsh 的孙进程会变孤儿继续占 3080（已在此前的审讯中论证，Windows 特有坑）。
- **补充**：App 退出钩子（`RunEvent::ExitRequested`）兜底，托盘退出与关窗退出共用同一终止路径；杀树以 spawn 出的 cmd.exe 为根。

### D5: 壳层状态机由 Rust 驱动，前端只渲染

状态：`loading → error{code,message} | setup | stopped | ready`。Rust 通过事件推给本地前端；就绪后 Rust 调 `window.navigate("http://127.0.0.1:3080")`。error 页重试 = 重跑完整启动序列（端口预检起）。回壳层页面（关窗询问、服务停止、升级中）通过 hash 导航（dev 用 Vite devUrl，release 用 tauri.localhost 源）。

- **理由**：生命周期逻辑留在 Rust 侧（可测试、不依赖页面 JS 存活）；前端保持无逻辑静态页。

### D6: 单实例用官方 tauri-plugin-single-instance

- **理由**：Windows 下命名管道/互斥量手写易错，官方插件成熟，回调里 `unminimize + set_focus`。
- **替代方案**：Rust 全局命名 Mutex —— 学习价值高但坑多，列为 v2 兴趣项。

### D7: 更新检查走 npm CLI，不用 HTTP 客户端

后台 `npm view @deepseek-ai/dsh version`（registry 最新）与 `dsh --version`（安装版）比对，纯 semver 字符串比较即可（版本格式 `0.1.0-rc.N`）。

- **理由**：npm 已随全局 dsh 安装成为环境前提；省掉 reqwest + JSON 解析 + registry API 契约维护。
- **失败语义**：任一命令失败 → 静默跳过（specs/version-management 已约束）。
- **升级流**：提示（原生对话框，避免打扰 service UI）→ 用户确认 → `npm i -g @deepseek-ai/dsh@<新版>` → 终止当前服务树 → 重启服务 → 重新轮询 → 导航。

### D8: 用户偏好（关闭行为）存 %APPDATA% 下 JSON

serde 手写读写 `{ "close_action": "ask" | "tray" | "exit" }`，放 Tauri `app_config_dir`。

- **理由**：单一小配置，不值得引插件；手写读写本身是学习点。
- **替代方案**：tauri-plugin-store —— 功能超出需求，暂不引入。

## Risks / Trade-offs

- **[壳崩溃但 dsh 存活 → 下次启动报端口占用]** → MVP 接受：错误页明示"端口被占"；v2 做存活 dsh 复用检测（探测端口上的服务是否为本客户端可识别的 dsh）。
- **[WebView 导航 3080 被 capabilities/CSP 拦截]** → 实现期第一件事验证 `allow-navigate`；兜底方案是 shell 前端 iframe 内嵌（改动面小，但牺牲 URL 直连语义）。
- **[升级重启打断进行中会话]** → 对话框明示"升级将重启服务"；用户可暂缓。
- **[npm 全局安装需要终端环境变量]** → 客户端 PATH 继承自资源管理器，npm 全局 bin 路径（本机 `C:\nvm4w\nodejs`）通常已在用户 PATH；若缺失，错误页提示运行 `npm i -g @deepseek-ai/dsh` 并附指导（specs/version-management 的 setup 页覆盖）。
- **[rc 版本迭代快，浏览器信任围栏（browser-trust）行为可能变]** → 客户端固定 `--host 127.0.0.1 --port 3080` 显式传参，不依赖默认值。

## Migration Plan

无历史系统：绿地项目，直接新建。回滚 = 卸载客户端 + `npm i -g @deepseek-ai/dsh@<旧版>`（npm 全局安装天然支持降级）。
