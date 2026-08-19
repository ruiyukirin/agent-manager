# PROJECT STATE

## 项目简介
Agent Manager —— Tauri 2 + React + Rust 桌面应用，用于检测、版本检查、更新和管理本机 AI Agent。

## 当前版本
0.1.2

## 技术栈
Tauri 2 + React + TypeScript + Vite + Rust

## 当前架构
- Rust 后端：adapter.rs 定义 AgentAdapter trait，各 Agent 独立适配器（codex/claude/hermes/openclaw/workbuddy/marvis）
- 前端：App.tsx 通过 Tauri invoke 调用后端命令
- 存储：storage.rs 保存 state.json / schedule.json 到 %APPDATA%\AgentManager

## 已完成功能
- 检测 6 个 AI Agent
- 版本检查（官方 API/页面）
- 更新机制（内置更新器 + 官方入口）
- 安装对话框（下载位置/安装位置选择流程）
- Marvis DirectDownload 接入
- Winget 安装分支
- Windows 计划任务定时检查
- 网络请求增加连接/总超时（GitHub API、npm、官方重定向、下载）
- 定时检查完成后通过 Windows 桌面通知反馈结果
- winget show 增加 45 秒超时包装，超时后终止子进程
- 计划任务名称统一，修复 /TR 引号与删除不存在任务时的错误判断
- install_agent 成功后重新检测并持久化状态，DirectDownload/Winget 不再假装“已安装”
- discover_agents 单 Agent 检测失败时以 Error 状态展示，而不是静默吞掉
- fallbackAgents 补齐 Claude，统一 Marvis publisher 与后端一致
- 移除硬编码 C:\Users\yuqil 路径，改用 USERPROFILE 定位 ~/.claude
- GitHub 发布
- GitHub Release 文案与 Tauri 实际 NSIS 产物名对齐，避免错误硬编码文件名
- 版本比较支持 beta/rc 预发布版本与稳定版本排序
- 清理未调用死代码 sha256 / download_to_temp / check_winget_available / run_installer_silent，并移除仅由此使用的 sha2 依赖

## 正在开发
- 自主完成阶段（网络/计划任务/安装状态/检测可靠性/一致性/可移植性/清理）已收尾

## 当前问题
- 真实 Winget / DirectDownload 端到端安装与更新未在本机实机验证（按要求不实际安装任何 Agent）

## 已知 Bug
- WebView2 窗口在 sandbox 下不显示（非项目 bug，是 Codex sandbox 对 WebView2 数据目录无写权限）

## 最近修改
- common.rs：GitHub/npm/重定向/下载 curl 增加 --connect-timeout 与 --max-time，新增 home_dir()
- claude.rs：使用 home_dir() 替代硬编码用户路径
- main.rs：统一计划任务名，修正 /TR 引号与删除错误判断；discover_agents 错误可见化；install_agent 成功后重新检测并 save_agents
- App.tsx：fallbackAgents 补齐 Claude，Marvis publisher 统一为 Tencent
- .gitignore：移除 Cargo.lock 忽略（应用应提交 lock），新增 *.bak/*.new/根目录截图忽略
- 清理 existing.rs.bak、main.rs.new、agent_manager_shot.jpg/png
- build-release.yml：修正 releaseName/releaseBody 重复 v 前缀
- install_agent 增加 InstallMethod::Winget 分支，复用 common.rs winget_install，不依赖 installer_path/install_dir
- Marvis Adapter 的 install_method 和 discover 返回值改为 DirectDownload，使用官方直链 https://marvis.qq.com/download/exe
- 确认 InstallMethod 前后端序列化统一为 camelCase，并通过 serde 序列化测试验证
- 修复 main.rs imports（adapter/model/state/Manager）
- 恢复完整 CSS（201 行）
- 定位 InstallMethod 序列化 bug
- 提交前修复：恢复 App.tsx 错误信息与 schedule 时间显示
- 提交前修复：Marvis fallback 对齐后端 DirectDownload
- 提交前修复：download_agent 对无法推断扩展名的 URL 使用 installer.exe 落盘
- 提交前修复：清理 existing.rs 末尾空行
- 自主收尾：is_update_available 改为先比较核心版本，再处理预发布版本与稳定版本排序，并新增 compares_prerelease_versions 测试
- 自主收尾：删除 common.rs 中未调用的 sha256 / download_to_temp / check_winget_available / run_installer_silent，移除 sha2 依赖
- 自主收尾：common.rs 新增 command_capture_stdout_timeout，winget show 增加 45 秒超时并补充超时测试
- 自主收尾：main.rs run_scheduled_check 接收 app identifier，通过 notify-rust 发送定时检查桌面通知
- 自主收尾：Cargo.toml 增加 notify-rust 直接依赖
- 自主收尾：build-release.yml releaseBody 改为通用 Assets 下载说明，避免错误产物文件名

## 下一步任务
- 等待人工进行 Git 提交确认（暂不 add/commit/push）
- 真实 Windows 环境端到端验证：Winget 安装、DirectDownload 静默安装、定时任务

## 重要技术决策
- 安装方式三态：Winget / DirectDownload / OpenBrowser
- Marvis 官方地址 https://marvis.qq.com/download/exe 是真实直链

## 禁止修改或需要特别注意的部分
- 无
