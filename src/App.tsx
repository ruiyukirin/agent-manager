import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { AgentInstance, AgentStatus, ScheduleConfig, UpdateResult } from './types'

const agentIcon: Record<string, string> = {
  codex: '/icons/codex.png',
  hermes: '/icons/hermes.png',
  openclaw: '/icons/openclaw.png',
  workbuddy: '/icons/workbuddy.png',
  marvis: '/icons/marvis.ico',
  claude: '/icons/claude.svg',
}

const fallbackAgents: AgentInstance[] = [
  {
    id: 'codex', name: 'Codex', publisher: 'OpenAI', installed: true, installPath: 'C:\\Program Files\\WindowsApps', executablePath: null,
    version: { product: '26.803.10989.0', component: null, bootstrap: null, channel: 'AppX', pe: '26.803.10989.0' }, latestVersion: null,
    status: 'checking', running: false, updateMode: 'native-updater', officialUrl: 'https://apps.microsoft.com/', detail: 'AppX 安装', lastChecked: null,
  },
  { id: 'hermes', name: 'Hermes Agent', publisher: 'Nous Research', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'native-updater', officialUrl: 'https://github.com/NousResearch/hermes-agent', detail: '未发现', lastChecked: null },
  { id: 'openclaw', name: 'OpenClaw', publisher: 'OpenClaw Foundation', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'native-updater', officialUrl: 'https://openclaw.ai', detail: '未发现', lastChecked: null },
  { id: 'workbuddy', name: 'WorkBuddy', publisher: 'Tencent', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'manual-action', officialUrl: 'https://workbuddy.ai/', detail: '官方 Windows x64 客户端', lastChecked: null },
  { id: 'marvis', name: 'Marvis', publisher: '腾讯科技（深圳）有限公司', installed: true, installPath: 'C:\\Program Files\\Tencent\\Marvis', executablePath: 'C:\\Program Files\\Tencent\\Marvis\\Application\\1.60.2100.153\\Marvis.exe', version: { product: '1.60.10.14', component: '1.60.2100.153', bootstrap: null, channel: 'Windows', pe: '1.60.2100.153' }, latestVersion: null, status: 'checking', running: false, updateMode: 'manual-action', officialUrl: 'https://marvis.qq.com/download/exe', detail: '检测到 MarvisUpdate.exe', lastChecked: null },
]

const statusLabel: Record<AgentStatus, string> = {
  'not-installed': '未安装',
  checking: '检查中',
  'up-to-date': '已是最新',
  'update-available': '可更新',
  'manual-action': '需手动操作',
  error: '检查失败',
}

function statusClass(status: AgentStatus): string {
  if (status === 'update-available' || status === 'manual-action') return 'status warning'
  if (status === 'error') return 'status danger'
  if (status === 'up-to-date') return 'status success'
  if (status === 'checking') return 'status neutral'
  return 'status muted'
}

function App() {
  const [agents, setAgents] = useState<AgentInstance[]>(fallbackAgents)
  const [selectedId, setSelectedId] = useState('marvis')
  const [loading, setLoading] = useState(true)
  const [working, setWorking] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const [schedule, setSchedule] = useState<ScheduleConfig>({ enabled: true, time: '02:00', agentIds: ['codex', 'hermes', 'openclaw', 'workbuddy', 'marvis'] })

  const loadAgents = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const result = await invoke<AgentInstance[]>('discover_agents')
      if (result.length) setAgents(result)
    } catch (e) {
      setError(`读取本机 Agent 失败：${String(e)}`)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadAgents(); void (async () => { try { setSchedule(await invoke<ScheduleConfig>('get_schedule')) } catch { /* defaults are safe */ } })() }, [loadAgents])

  const selected = agents.find((agent) => agent.id === selectedId) ?? agents[0]
  const installedCount = useMemo(() => agents.filter((agent) => agent.installed).length, [agents])
  const updateCount = useMemo(() => agents.filter((agent) => agent.status === 'update-available' || agent.status === 'manual-action').length, [agents])

  async function check(agentId: string) {
    setWorking(agentId); setError(''); setMessage('')
    try {
      const result = await invoke<AgentInstance>('check_updates', { agentId })
      setAgents((current) => current.map((agent) => agent.id === agentId ? result : agent))
      setMessage(`${result.name} 检查完成`)
    } catch (e) {
      setError(`检查失败：${String(e)}`)
    } finally { setWorking(null) }
  }

  async function update(agentId: string) {
    setWorking(agentId); setError(''); setMessage('')
    try {
      const result = await invoke<UpdateResult>('update_agent', { agentId })
      setMessage(result.message)
      if (result.officialUrl && result.mode === 'manual-action') {
        setMessage(`${result.message} 可以在详情页打开官方入口。`)
      }
      await loadAgents()
    } catch (e) {
      setError(`更新失败：${String(e)}`)
    } finally { setWorking(null) }
  }

  async function openOfficial(agentId: string) {
    try { await invoke('open_official_url', { agentId }) } catch (e) { setError(`打开官方入口失败：${String(e)}`) }
  }

  async function configureSchedule() {
    const time = window.prompt('每日检查时间（HH:mm）', schedule.time)?.trim()
    if (!time || !/^([01]\d|2[0-3]):[0-5]\d$/.test(time)) { setError('请输入有效时间，例如 02:00'); return }
    const next = { ...schedule, time, enabled: true }
    setSchedule(next)
    try { await invoke('set_schedule', { config: next }); setMessage(`检查计划已保存：每天 ${time}`) } catch (e) { setError(`保存计划失败：${String(e)}`) }
  }
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><div className="brand-mark">A</div><div><h1>Agent Manager</h1><p>本地 AI Agent 管家</p></div></div>
        <div className="sidebar-section"><span className="section-label">工作台</span><button className="nav-item active" onClick={() => setSelectedId(selected.id)}><span>▦</span> Agent 总览</button><button className="nav-item" onClick={() => void configureSchedule()}><span>◷</span> 检查计划</button><button className="nav-item" onClick={() => setMessage('日志会记录在本机，不会上传')}><span>≡</span> 操作日志</button></div>
        <div className="sidebar-bottom"><div className="privacy"><span>●</span><div><strong>本地优先</strong><p>版本、配置和日志仅保存在本机</p></div></div><div className="app-version">Agent Manager v0.1.0</div></div>
      </aside>

      <main className="main-content">
        <header className="topbar"><div><p className="eyebrow">OVERVIEW / 2026-08-14</p><h2>早上好，Agent 管家已就绪</h2></div><button className="primary-button" onClick={() => void loadAgents()} disabled={loading}>{loading ? '扫描中…' : '重新扫描'}</button></header>
        {error && <div className="alert error-alert"><span>!</span>{error}<button onClick={() => setError('')}>×</button></div>}
        {message && <div className="alert success-alert"><span>✓</span>{message}<button onClick={() => setMessage('')}>×</button></div>}

        <section className="stats-grid"><div className="stat-card"><span className="stat-icon blue">⌘</span><div><p>已发现 Agent</p><strong>{installedCount}<small> / {agents.length}</small></strong></div><span className="stat-trend">本机</span></div><div className="stat-card"><span className="stat-icon orange">↻</span><div><p>可处理更新</p><strong>{updateCount}<small> 项</small></strong></div><span className="stat-trend">需确认</span></div><div className="stat-card"><span className="stat-icon green">✓</span><div><p>最近检查</p><strong>刚刚</strong></div><span className="stat-trend">已启用</span></div></section>

        <section className="content-grid">
          <div className="agent-list-panel panel"><div className="panel-heading"><div><h3>本机 Agent</h3><p>共 {agents.length} 个适配器 · Windows 11</p></div><button className="icon-button" onClick={() => void loadAgents()}>↻</button></div><div className="agent-list">{agents.map((agent) => <button key={agent.id} className={`agent-row ${selected?.id === agent.id ? 'selected' : ''}`} onClick={() => setSelectedId(agent.id)}><div className={`agent-avatar ${agent.id}`}><img className="agent-icon-img" src={agentIcon[agent.id] ?? ""} alt={agent.name} onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; (e.target as HTMLImageElement).parentElement!.textContent = agent.name.slice(0, 1) }} /></div><div className="agent-row-copy"><div><strong>{agent.name}</strong>{agent.installed && <span className="dot-installed" title="已安装" />}</div><span>{agent.detail}</span></div><div className="agent-row-meta"><span className={statusClass(agent.status)}>{statusLabel[agent.status]}</span>{agent.running && <span className="running-dot">运行中</span>}</div></button>)}</div></div>

          {selected && <div className="detail-panel panel"><div className="panel-heading"><div className="detail-heading-with-icon"><img className="detail-icon-img" src={agentIcon[selected.id] ?? ""} alt={selected.name} onError={(e) => { (e.target as HTMLImageElement).style.display = "none" }} /><div><p className="eyebrow">AGENT DETAIL</p><h3>{selected.name}</h3></div></div><span className="publisher-chip">{selected.publisher}</span></div>{selected.installed ? <><div className="version-hero"><div><span>当前版本</span><strong>{selected.version?.product ?? '未识别'}</strong>{selected.version?.component && selected.version.component !== selected.version?.product && <small>组件 {selected.version.component}</small>}{selected.version?.pe && selected.version.pe !== selected.version?.product && <small>PE {selected.version.pe}</small>}</div><div className="version-arrow">→</div><div><span>最新版本</span><strong className={selected.latestVersion ? 'version-highlight' : ''}>{selected.latestVersion ?? '等待检查'}</strong><small>{selected.status === 'manual-action' ? '官方入口可更新' : '用户确认后更新'}</small></div></div><div className="detail-info"><div><span>安装位置</span><code>{selected.installPath ?? '未知'}</code></div><div><span>执行文件</span><code>{selected.executablePath ?? selected.installPath ?? '未发现'}</code></div><div><span>更新方式</span><strong>{selected.updateMode === 'manual-action' ? '官方入口 / 手动操作' : '内置更新器'}</strong></div><div><span>运行状态</span><strong>{selected.running ? '运行中' : '未运行'}</strong></div></div><div className="action-row"><button className="primary-button" onClick={() => void check(selected.id)} disabled={working === selected.id}>{working === selected.id ? '处理中…' : selected.latestVersion ? '检查更新' : '检查版本'}</button>{selected.status === 'manual-action' && <button className="secondary-button" onClick={() => void openOfficial(selected.id)}>打开官方入口</button>}{selected.status === 'update-available' && <button className="secondary-button" onClick={() => void update(selected.id)}>确认更新</button>}</div></> : <div className="empty-detail"><div className="empty-icon">＋</div><h4>尚未检测到 {selected.name}</h4><p>安装后重新扫描，管理器会自动识别版本和更新。</p><button className="secondary-button" onClick={() => void openOfficial(selected.id)}>打开官方下载</button></div>}</div>}
        </section>
        <section className="notice"><span>ⓘ</span><p><strong>安全更新策略</strong>　每次更新都会先创建本机备份，并按照更新前运行状态决定是否重启。WorkBuddy 和 Marvis 的静默安装参数尚未在本机验证时，会提供官方入口而不是误报更新成功。</p></section>
      </main>
    </div>
  )
}

export default App






