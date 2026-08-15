# Agent Manager

本地 AI Agent 发现与更新管理工具。

检测本机安装的 AI 编码助手（Claude、Codex、WorkBuddy、Marvis 等），识别版本、检查更新、备份配置，统一管理入口。

## 功能

- 自动发现已安装的 AI Agent
- 版本识别（注册表 / PE / 路径分析）
- 检查可用更新
- 更新前自动备份配置
- 更新后按需重启 Agent
- 定时检查计划

## 技术栈

- **前端**: React + TypeScript + Vite
- **后端**: Rust + Tauri 2
- **平台**: Windows（AppX / 桌面安装 / CLI 安装）

## 开发

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run build
npm run tauri build
```
