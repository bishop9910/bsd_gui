# BSD 下载器（GUI）

给 C++20 多线程下载核心 `bsd` 套的 Windows 桌面界面：Tauri 2 + Vue 3 + Naive UI，
通过原生 `LoadLibraryExW` 动态加载 `bsd_core.dll`，不额外引入任何 C++/Rust 运行时依赖。

## 功能

- 多线程分片下载、断点续传（`.part` / `.meta`）、HTTP/HTTPS、代理、自定义请求头与 User-Agent
- MD5 / SHA-1 / SHA-256 / SHA-512 校验，失败会删除文件并给出明确原因
- 全局限速（字节/秒）、超时与重试次数可调，重试由 GUI 记录次数并显示第几次
- 任务列表：实时进度、速度、剩余时间、已下载大小、错误信息、完成后的文件位置
- 批量粘贴多个链接一次创建多个任务，每个任务独立句柄、可并行下载，也可单独取消
- 运行日志：按级别（TRACE–ERROR）着色、级别筛选、关键字搜索、复制、清空
- 设置项与任务记录保存在 localStorage，重启后仍可看到历史任务（未完成的标记为“已中断”，可重试）
- 核心加载失败时，界面顶部横幅 + “关于”页会列出失败原因与所有尝试过的 DLL 路径

## 运行

```powershell
pnpm install
pnpm tauri dev      # 开发模式（前端热更新 + Rust 自动重编）
pnpm tauri build    # 打包安装包（release + LTO，耗时较长）
```

`pnpm build` 只做类型检查与前端打包（`vue-tsc --noEmit && vite build`）。

## 核心 DLL 的查找顺序

`bsd_core.dll` 及其依赖（`fmt`、`libcurl`、`libcrypto-3-x64`、`libssl-3-x64`、`z`、`legacy`）
需要放在同一目录下，运行时按以下顺序查找：

1. 可执行文件同目录 `bsd_core.dll`
2. 可执行文件同目录 `binaries\bsd_core.dll`
3. 安装包资源目录 `binaries\bsd_core.dll`（打包时由 `bundle.resources` 写入）
4. 安装包资源目录 `bsd_core.dll`
5. 编译期 `src-tauri\binaries\bsd_core.dll`（开发时的兜底）

`src-tauri\build.rs` 会在每次 `cargo build` 时把 `src-tauri\binaries\*.dll` 复制到
`target\<profile>\`，因此 `tauri dev` 直接可用。加载时使用
`LOAD_WITH_ALTERED_SEARCH_PATH`，依赖 DLL 从 `bsd_core.dll` 所在目录解析。

新增/替换 DLL 后重新执行一次 `cargo build`（或 `pnpm tauri dev`）即可，构建脚本按
文件大小与修改时间判断是否需要覆盖。

## 目录结构

```
src/
  AppShell.vue          侧边栏 + 路由出口 + 事件订阅 + 核心状态横幅
  App.vue               Naive UI 主题 / 消息 / 对话框 / 通知 Provider
  theme.ts              Naive UI 深色主题覆盖
  styles/global.css     设计令牌与通用样式
  lib/bsd.ts            invoke 命令与事件监听封装
  lib/store.ts          设置、任务、日志的响应式存储（localStorage 持久化）
  lib/types.ts          与 Rust 端 serde 结构一一对应的类型
  lib/format.ts         字节/速度/时长/时间格式化
  components/           AppIcon（内联 SVG）、StatusChip、TaskCard
  views/                DownloadsView / SettingsView / LogsView / AboutView
src-tauri/
  src/ffi.rs            bsd_core.dll 的零依赖 Win32 动态加载与 C ABI 绑定
  src/download.rs       参数校验、句柄生命周期、下载线程、事件上报
  src/lib.rs            Tauri 命令、核心状态、日志回调、目录选择
  binaries/             随包分发的 bsd_core.dll 及其依赖
  build.rs              构建时把 binaries/*.dll 复制到 exe 同目录
```

## 常见问题

| 现象 | 原因与处理 |
| --- | --- |
| 顶部提示“核心未加载” | 展开横幅或到“关于”页查看具体错误与已尝试的路径；确认 DLL 齐全且同为 64 位 |
| 下载报“参数错误” | 校验值必须是 `算法:十六进制`（界面按算法+值分开填写）；文件名不能带 `\` 或 `/` |
| 断点续传不生效 | 服务器需要支持 `Range`；缺少 `ETag`/`Last-Modified` 时核心会拒绝续传 |
| 取消要等一会儿 | 取消是协作式的：当前分片请求返回后才会停止，通常 1 秒内生效 |
| 目录选择器打不开 | 由系统 `powershell -STA` 弹出 FolderBrowserDialog，若被安全软件拦截可手动输入路径 |
