# Agent Manager

**检测、查版本、更新你 Windows 上装的 AI 编码助手。**

[English](README.md) · 中文 · 作者：**Kirin** · [MIT 许可证](LICENSE)

![平台](https://img.shields.io/badge/%E5%B9%B3%E5%8F%B0-Windows%2010%2F11-0078D6)
![许可证](https://img.shields.io/badge/license-MIT-green)

![Agent Manager](docs/screenshot.png)

## 为什么做这个

AI 编码助手更新得飞快，装法还五花八门——微软商店（AppX）、winget（微软包管理器）、npm（Node 包管理器）、普通安装包，或者各自的内嵌更新器。想知道本机装了哪些、都是什么版本、有没有漏掉更新，只能手动一个个敲 `--version`，再去各家官网翻。

Agent Manager 扫一遍本机，把所有信息集中在一个窗口里。

## 支持的 Agent

| Agent | 靠什么找到安装 | 当前版本从哪来 | 更新从哪查 |
|---|---|---|---|
| Claude | AppX、运行中的进程、卸载表、常见路径、递归扫描 | 内嵌版本 / 安装路径 | winget `Anthropic.Claude` |
| Codex | AppX、`%LOCALAPPDATA%\OpenAI\Codex` 下的 CLI 安装、运行中的进程 | `codex --version` | winget `OpenAI.Codex` |
| Hermes Agent | 递归扫描 `%LOCALAPPDATA%\hermes` | `hermes --version`（日期版本） | GitHub 发布页 |
| OpenClaw | npm 全局命令 | `openclaw --version` | npm 源 |
| WorkBuddy | 卸载表、Program Files、`%LOCALAPPDATA%` | 卸载表 `DisplayVersion` | winget `Tencent.WorkBuddy` |
| DeepSeek Harness | 卸载表（键名是随机 GUID） | 卸载表 `DisplayVersion` | 官方更新源（nightly 通道） |

## 功能

- 自动发现已安装的 Agent，展示安装位置、可执行文件和运行状态
- 所有 Agent 走同一条版本探测链：程序自报 → 卸载表声明 → 内嵌 PE 版本 → 安装路径
- 更新检查直连各家真实来源：winget、npm、GitHub 发布页，或厂商自己的更新源
- 安装辅助：执行 winget 包、把官方安装包下载到你指定的目录，或打开官方下载页
- 通过 Windows 计划任务做无窗口的每日定时检查，结果以桌面通知反馈
- 本地优先：状态和日志只存在 `%APPDATA%\AgentManager`，不上传任何数据

## 下载

到 [**Releases**](https://github.com/ruiyukirin/agent-manager/releases) 拿最新安装包：

| 文件 | 说明 |
|---|---|
| `Agent.Manager_<版本>_x64-setup.exe` | NSIS 安装包 —— 推荐 |
| `Agent.Manager_<版本>_x64_en-US.msi` | MSI 安装包 |

**运行要求**：Windows 10 / 11（x64）。需要 WebView2 运行时 —— Windows 11 和较新的 Windows 10 已自带。

## 目前的局限（v0.1.x）

做不到的事情，这里说清楚：

- **只支持 Windows。** 检测依赖注册表、`tasklist`、`schtasks` 和 `winget`。
- **首次扫描要几十秒。** 每个 Agent 都要查注册表、查进程、跑 `--version`；扫描完成后列表才填充。
- **只报告更新，不就地安装。** 每一行会告诉你要执行什么命令，或打开官方下载页。程序不会静默覆盖你的文件。
- **部分静默安装参数还没做过端到端验证。** 遇到无法确认的情况，程序会打开官方下载页，而不是假装更新成功。
- **更新前备份只在"更新确实会改动文件"时才触发** —— 目前所有 Agent 都只是提示你手动更新，所以实际上不会执行。

## 开发

```bash
npm install
npm run tauri dev
```

构建与测试：

```bash
npm run build        # 前端类型检查 + 构建
npm test             # 前端测试（目前还没有）
npm run check-version
cd src-tauri && cargo test
```

前端：React + TypeScript + Vite。后端：Rust + Tauri 2。每个 Agent 一个适配器，放在 `src-tauri/src/adapters/` 下。

## 许可证

MIT © 2026 Kirin
