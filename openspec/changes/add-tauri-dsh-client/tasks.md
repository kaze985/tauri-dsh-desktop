## 1. 脚手架与环境

- [x] 1.1 确认 Rust 工具链（rustc/cargo）与本机 Node/npm 版本满足 Tauri 2 要求，缺则安装
- [x] 1.2 用 create-tauri-app 在本仓库生成项目（vanilla TS + Vite 前端），验证 `cargo check` 与前端 dev 构建均通过
- [x] 1.3 配置 tauri.conf.json：窗口标题/尺寸、应用标识、默认窗口隐藏控制台行为

## 2. 壳层前端（本地状态页）

- [x] 2.1 实现 loading 页与错误页（含重试/退出按钮、错误原因文案），页面间切换由 Rust 事件驱动
- [x] 2.2 打通 Rust → 前端状态事件（loading / error{code,message} / ready）

## 3. 服务生命周期（Rust）

- [x] 3.1 实现端口预检：探测 127.0.0.1:3080，占用则进错误态（端口被占 + 退出）
- [x] 3.2 实现 dsh CLI 可用性检查（`dsh --version`），缺失时展示 setup 指导页
- [x] 3.3 以 CREATE_NO_WINDOW spawn `dsh web --host 127.0.0.1 --port 3080`，记录子进程句柄
- [x] 3.4 就绪轮询（每 300ms TCP 探测，30s 超时）；就绪后 `window.navigate("http://127.0.0.1:3080")`
- [ ] 3.5 验证并配置 Tauri capabilities 允许导航到外部 URL（core:webview:allow-navigate）
- [x] 3.6 监听子进程意外退出：应用运行期 dsh 崩溃 → 窗口回到本地错误页（重启/退出按钮）
- [x] 3.7 干净终止：所有退出路径统一走 `taskkill /PID <pid> /T /F`，并挂 App 退出钩子兜底

## 4. 托盘与关闭语义

- [x] 4.1 添加托盘图标与菜单：显示主窗口 / 退出 / 检查更新 / 重置关闭行为
- [x] 4.2 拦截窗口关闭：对话框提供【退出程序 / 最小化到托盘】+【不再询问】复选框
- [x] 4.3 关闭行为偏好持久化到 app_config_dir 的 JSON（ask/tray/exit），托盘可重置

## 5. 单实例

- [x] 5.1 集成 tauri-plugin-single-instance：二次启动聚焦已有窗口（含托盘隐藏时恢复）后退出

## 6. 版本管理

- [x] 6.1 后台比对 `npm view @deepseek-ai/dsh version` 与 `dsh --version`，网络失败静默跳过
- [x] 6.2 发现新版时非阻塞通知（原生对话框，明示升级会重启服务），用户确认后执行 `npm i -g @deepseek-ai/dsh@<新版>`
- [x] 6.3 升级成功后终止旧服务树 → 重启服务 → 重新轮询就绪 → 重新导航

## 7. 验证与收尾

- [ ] 7.1 三个必测场景：正常启动；3080 被占用；断网启动（更新检查静默跳过、服务照常）
- [x] 7.2 验证退出后无孤儿进程：退出后 `tasklist` 无残留 node/dsh 且 3080 已释放
- [ ] 7.3 `cargo tauri build` 出 release 安装包，安装后完整走一遍 DoD（双击 → UI → 关窗询问 → 杀进程）
- [x] 7.4 应用图标与窗口元信息收尾（可选）
