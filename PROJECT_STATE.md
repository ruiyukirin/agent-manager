# PROJECT STATE

## 项目简介
Agent Manager —— Tauri 2 + React + Rust 桌面应用，用于检测、版本检查、更新和管理本机 AI Agent。

## 当前版本
0.1.2

## 技术栈
Tauri 2 + React + TypeScript + Vite + Rust

## 项目位置（2026-09-12 迁移）
- 当前开发位置：`D:\DeepSeek Harness\软件\agent-manager`（由 DeepSeek Harness 接管开发）
- 旧位置：`D:\AIxiangmu\codex\ruanjian\agent-manager`（仅保留归档，不再修改）
- 复制范围：全部源码 + `.git`（保留完整提交历史与 origin 远端），排除 `node_modules/`、`src-tauri/target/`、`dist/`、`release/`
- 注意：依赖与编译产物未随迁移复制，首次开发前需 `npm install`（Rust 侧 `cargo` 会自动重建 target）

## 当前架构
- Rust 后端：adapter.rs 定义 AgentAdapter trait，各 Agent 独立适配器（codex/claude/hermes/openclaw/workbuddy/deepseek）
- 版本探测：common.rs 的 probe_version（统一探测链）+ normalize_version（归一化，禁止出现 Some("")）
- 前端：App.tsx 通过 Tauri invoke 调用后端命令
- 存储：storage.rs 保存 state.json / schedule.json 到 %APPDATA%\AgentManager

## 已完成功能
- 检测 6 个 AI Agent
- 版本检查（官方 API/页面）
- 更新机制（内置更新器 + 官方入口）
- 安装对话框（下载位置/安装位置选择流程）
- DeepSeek Harness DirectDownload 接入（官方 nightly feed 直链）
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
- 统一版本探测链（程序自报 → 卸载表声明 → PE 内嵌 → 路径推断）+ 版本号归一化，修复 Codex / Hermes / WorkBuddy 三个真机缺陷
- 修复 WorkBuddy / Hermes 安装位置错位（新增 install_root_for，不再写死父目录层级）
- Hermes 按日期版本比较（云端标签 v2026.9.24 体系），内部 semver 放入 component 仅作展示
- CI 拆两段：ci.yml 每次推送跑版本一致性 + 前端构建 + cargo test；build-release.yml 只在打标签时出安装包
- 新增 scripts/check-version.mjs 发版防错（拦住标签与产物版本不一致，v0.1.1 曾因此产出名为 0.1.0 的安装包）
- 备份策略修正：不再备份可执行程序（GUI 程序动辄几百 MB）、凭据类文件一律排除、只在适配器真的会改文件时才备份
- App.tsx 移除编造的「已安装」占位数据，改为诚实空状态，真实数据只来自 discover_agents

## 正在开发
- 自主完成阶段（网络/计划任务/安装状态/检测可靠性/一致性/可移植性/清理）已收尾

## 当前问题
- 真实 Winget / DirectDownload 端到端安装与更新未在本机实机验证（按要求不实际安装任何 Agent）
- Hermes 的日期版本体系是从官方标签推断的（v2026.9.24 / v2026.9.21 / v2026.9.14 …），若上游改成 semver 需同步调整 hermes_versions()
- README 两处不实描述已随双语改写修正（「更新前自动备份」「更新后按需重启」改为如实说明）
- 界面打开后所有行停在「检查中」、「最新版本」显示「等待检查」：程序不会自动检查更新，需用户逐个点「检查版本」；且状态文案「检查中」并不准确，实际是「尚未检查」
- 顶栏日期硬编码为 `OVERVIEW / 2026-08-14`，不随当前时间变化
- 首次扫描约 20~40 秒（每个适配器都要查注册表、跑 PowerShell 读版本、查进程），期间界面显示占位数据

## 已知 Bug
- WebView2 窗口在 sandbox 下不显示（非项目 bug，是 Codex sandbox 对 WebView2 数据目录无写权限）

## 最近修改
- 新增 LICENSE（MIT，Copyright (c) 2026 Kirin）
- 作者署名：package.json 的 author/license、Cargo.toml 的 authors、19 个源码文件头部 `// Author: Kirin`
- README 重写为双语：README.md（英文，默认）+ README.zh-CN.md，顶部互加语言切换；补下载入口、截图、支持清单，并把两处不实描述改成如实说明
- 新增 docs/screenshot.png（1440x900，PNG）
- 仓库元数据：英文简介、主页指向 Releases、15 个话题标签
- 版本探测根因修复：common.rs 新增 normalize_version / VersionProbe / probe_version / install_root_for / normalize_date_version，各适配器改为走统一探测链
- Codex：CLI 分支与 Programs 分支补上 codex.exe --version（此前当前版本恒为空，明明有更新也报不出来）
- Hermes：改用绝对路径执行 hermes.exe --version（此前用裸文件名靠 PATH，只取到空串），并按日期版本比较
- WorkBuddy：版本改从卸载表 DisplayVersion 取（GUI 程序不执行 --version），安装位置改用 install_root_for 定位
- DeepSeek Harness：版本改走探测链，官方 feed 加进程内缓存（避免把网络请求带进扫描路径）
- backup.rs：不再备份可执行程序，新增 is_secret 过滤凭据文件，并补 2 个测试
- main.rs / adapter.rs：新增 update_mutates_files()，只在适配器真的会改本机文件时才做更新前备份
- CI：新增 ci.yml（推送即跑版本一致性 + 前端构建 + cargo test）；build-release.yml 增加标签版本校验步骤
- scripts/check-version.mjs：新增发版防错脚本（用 v0.1.1 的历史事故当反面用例验证通过）
- App.tsx：删除编造的「已安装」占位数据，改为 placeholder() 诚实空状态
- 真机复验：cargo test 20 项全过、前端构建通过、--scheduled-check 跑通，Codex 0.155.0-alpha.9.2 / Hermes 2026.9.14 / WorkBuddy 5.6.2 / DeepSeek 0.2.0-rc.2 全部识别正确
- 移除 Marvis：删除 MarvisAdapter、public/icons/marvis.ico、backup.rs 内的 Marvis 备份项、计划任务默认项与全部硬编码引用
- 新增 DeepSeek Harness 适配器（adapters/deepseek.rs）：遍历卸载表取当前版本、读官方 nightly feed 查最新版本与安装包直链、DirectDownload + 官方入口
- common.rs 新增 parse_uninstall_entry / uninstall_entry（应对随机 GUID 卸载键）与 parse_feed_version / parse_feed_installer（electron-updater feed 解析），并补 2 个单测
- App.tsx：图标映射、fallbackAgents、默认选中项、计划任务默认项、安全策略提示文案同步为 DeepSeek Harness
- 真机验证：前端 tsc + vite build 通过，cargo test 11 项全过，--scheduled-check 无窗口跑通（39.6 秒，检测到 4 个已安装 Agent）
- 前端侧边栏品牌标识从写死的「A」改为选定章鱼 Logo（public/icons/agent-manager.png）
- tauri.conf.json：应用标识 com.tencent.agentmanager 改为 com.ruiyukirin.agentmanager
- src-tauri/icons/icon.ico：替换为选定「章鱼剪影 + 眼睛」Logo，宝蓝色（内含 16/24/32/48/64/128/256 多尺寸）
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
- 对外发布准备（剩余）：应用内自动更新（tauri-plugin-updater）、代码签名证书
- 跨平台暂不做：当前为 Windows 专用（reg.exe / tasklist / schtasks / winget / PE 版本）

## 重要技术决策
- 安装方式三态：Winget / DirectDownload / OpenBrowser
- 截图方法（踩过坑）：窗口被遮挡时 Chromium 会停止重绘，此时抓屏（CopyFromScreen）会拍到别的窗口、PrintWindow 只能拿到旧帧。正确做法是用 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 打开调试端口，再用 CDP 的 `Page.captureScreenshot` 由渲染器直出
- 版本探测统一走 common.rs 的 probe_version，优先级：程序自报（--version）> 卸载表 DisplayVersion > 可执行文件 PE 版本 > 路径推断；GUI 程序（WorkBuddy / DeepSeek Harness）不执行 --version，避免误弹窗口
- 备份只备配置不备二进制，凭据类文件（credential / secret / token / password / .key / .pem / cookie）一律排除；`~/.dsh` 体积太大且含凭据，明确不纳入
- DeepSeek Harness 接入依据（均为本机实测）：卸载键名是随机 GUID，版本靠遍历 HKCU/HKLM 卸载表取 DisplayVersion（当前 0.2.0-rc.2，与卸载程序 exe、官方 feed 三处一致）
- DeepSeek Harness 最新版本源：官方 feed `https://download.deepseek.com/dsh-desk/feeds/win-x64/nightly.yml`（与安装目录 resources/app-update.yml 的 provider generic / channel nightly 一致），含版本号与安装包直链
- winget 里没有官方 DeepSeek Harness 包（搜索结果只有第三方仿制包 DSH Desktop，不可使用），因此安装方式定为 DirectDownload + feed 直链
- DeepSeek Harness 安装包约 289 MB；用户数据 `~/.dsh` 达 1.2 GB 且含 `.credentials.yaml`，故未纳入 backup.rs 备份范围
- Marvis 已整体移除（适配器、public/icons/marvis.ico、备份项、计划任务默认项及全部硬编码引用）

## 禁止修改或需要特别注意的部分
- 无
