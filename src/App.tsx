import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { AgentInstance, AgentStatus, InstallResult, ScheduleConfig, UpdateResult } from './types'

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
    status: 'checking', running: false, updateMode: 'native-updater', officialUrl: 'https://apps.microsoft.com/', detail: 'AppX 安装', lastChecked: null, installUrl: 'https://apps.microsoft.com/', installMethod: { winget: { packageId: 'OpenAI.Codex' } },
  },
  { id: 'claude', name: 'Claude', publisher: 'Anthropic', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'manual-action', officialUrl: 'https://www.anthropic.com/claude-code', detail: '未发现', lastChecked: null, installUrl: 'https://www.anthropic.com/claude-code', installMethod: { openBrowser: { url: 'https://www.anthropic.com/claude-code' } } },
  { id: 'hermes', name: 'Hermes Agent', publisher: 'Nous Research', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'native-updater', officialUrl: 'https://github.com/NousResearch/hermes-agent', detail: '未发现', lastChecked: null, installUrl: 'https://github.com/NousResearch/hermes-agent', installMethod: { openBrowser: { url: 'https://github.com/NousResearch/hermes-agent' } } },
  { id: 'openclaw', name: 'OpenClaw', publisher: 'OpenClaw Foundation', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'native-updater', officialUrl: 'https://openclaw.ai', detail: '未发现', lastChecked: null, installUrl: 'https://openclaw.ai', installMethod: { openBrowser: { url: 'https://openclaw.ai' } } },
  { id: 'workbuddy', name: 'WorkBuddy', publisher: 'Tencent', installed: false, installPath: null, executablePath: null, version: null, latestVersion: null, status: 'not-installed', running: false, updateMode: 'manual-action', officialUrl: 'https://workbuddy.ai/', detail: '官方 Windows x64 客户端', lastChecked: null, installUrl: 'https://workbuddy.ai/', installMethod: { openBrowser: { url: 'https://workbuddy.ai/' } } },
  { id: 'marvis', name: 'Marvis', publisher: 'Tencent', installed: true, installPath: 'C:\\Program Files\\Tencent\\Marvis', executablePath: 'C:\\Program Files\\Tencent\\Marvis\\Application\\1.60.2100.153\\Marvis.exe', version: { product: '1.60.10.14', component: '1.60.2100.153', bootstrap: null, channel: 'Windows', pe: '1.60.2100.153' }, latestVersion: null, status: 'checking', running: false, updateMode: 'manual-action', officialUrl: 'https://marvis.qq.com/download/exe', detail: '检测到 MarvisUpdate.exe', lastChecked: null, installUrl: 'https://marvis.qq.com/download/exe', installMethod: { directDownload: { url: 'https://marvis.qq.com/download/exe' } } },
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

type StepName =
  | 'confirm'
  | 'select-download'
  | 'downloading'
  | 'select-install'
  | 'installing'
  | 'done'
  | 'failed'

function App() {
  const [agents, setAgents] = useState<AgentInstance[]>(fallbackAgents)
  const [selectedId, setSelectedId] = useState('marvis')
  const [loading, setLoading] = useState(true)
  const [working, setWorking] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const [schedule, setSchedule] = useState<ScheduleConfig>({ enabled: true, time: '02:00', agentIds: ['codex', 'hermes', 'openclaw', 'workbuddy', 'marvis'] })
  const [installTarget, setInstallTarget] = useState<AgentInstance | null>(null)
  const [step, setStep] = useState<StepName | null>(null)
  const [stepMessage, setStepMessage] = useState('')

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

  /* ── Install flow ───────────────────────────────────────── */

  function requestInstall(agent: AgentInstance) {
    setInstallTarget(agent)
    setStep('confirm')
    setStepMessage('')
  }

  function closeInstall() {
    if (step === 'downloading' || step === 'installing') return
    setInstallTarget(null)
    setStep(null)
    setStepMessage('')
  }

  async function handleConfirm() {
    if (!installTarget) return
    const m = installTarget.installMethod

    if ('winget' in m) {
      setStep('downloading')
      setWorking(installTarget.id)
      try {
        const result = await invoke<InstallResult>('install_agent', {
          agentId: installTarget.id,
          installerPath: '',
          installDir: '',
        })
        setMessage(result.message)
        setStep('done')
        await loadAgents()
      } catch (e) { setStep('failed'); setStepMessage(String(e)) }
      finally { setWorking(null) }
      return
    }

    if ('openBrowser' in m) {
      try {
        await invoke('open_official_url', { agentId: installTarget.id })
        setStep('done')
        setMessage(`已打开 ${installTarget.name} 下载页面`)
      } catch (e) { setStep('failed'); setStepMessage(String(e)) }
      return
    }

    // DirectDownload: start path selection
    setStep('select-download')
  }

  async function pickDownloadPath() {
    if (!installTarget) return
    const dir = await open({ directory: true, title: `选择 ${installTarget.name} 安装包保存位置` })
    if (!dir) { setStep('confirm'); return }
    setStep('downloading')
    setWorking(installTarget.id)
    try {
      const installerPath = await invoke<string>('download_agent', {
        agentId: installTarget.id,
        downloadDir: dir,
      })
      // Download done, ask for install directory
      const installDir = await open({ directory: true, title: `选择 ${installTarget.name} 安装位置` })
      if (!installDir) {
        setStep('failed')
        setStepMessage(`已取消安装，安装文件保存在：${installerPath}`)
        setWorking(null)
        return
      }
      setStep('installing')
      const result = await invoke<InstallResult>('install_agent', {
        agentId: installTarget.id,
        installerPath,
        installDir,
      })
      setMessage(result.message)
      setStep('done')
      await loadAgents()
    } catch (e) { setStep('failed'); setStepMessage(String(e)) }
    finally { setWorking(null) }
  }

  /* ── Render dialog ──────────────────────────────────────── */

  function renderDialog() {
    if (!installTarget || !step) return null

    const btnClose = <button className="modal-close" onClick={closeInstall}>×</button>
    const btnCancel = <button className="secondary-button" onClick={closeInstall}>取消</button>
    const btnDone = <button className="primary-button" onClick={closeInstall}>关闭</button>

    if (step === 'confirm') {
      const m = installTarget.installMethod
      const lines: { l: string; v: string }[] = []
      if ('winget' in m) lines.push({ l: '安装方式', v: 'Winget 包管理器' }, { l: '包 ID', v: m.winget.packageId })
      else if ('directDownload' in m) lines.push({ l: '安装方式', v: '下载安装包并静默安装' }, { l: '下载地址', v: m.directDownload.url })
      else if ('openBrowser' in m) lines.push({ l: '安装方式', v: '浏览器打开官方下载页' }, { l: '地址', v: m.openBrowser.url })

      return (
        <div className="modal-overlay" onClick={closeInstall}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header"><h3>安装 {installTarget.name}</h3>{btnClose}</div>
            <div className="modal-body">
              <table className="install-detail-table"><tbody>
                {lines.map((l) => <tr key={l.l}><td className="install-detail-label">{l.l}</td><td className="install-detail-value">{l.v}</td></tr>)}
              </tbody></table>
              {'directDownload' in m && <p className="install-path-hint">安装过程将依次选择下载位置和安装目录。</p>}
            </div>
            <div className="modal-footer">
              {btnCancel}
              <button className="primary-button" onClick={() => void handleConfirm()}>确认安装</button>
            </div>
          </div>
        </div>
      )
    }

    if (step === 'select-download') {
      return (
        <div className="modal-overlay" onClick={closeInstall}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header"><h3>选择下载位置</h3>{btnClose}</div>
            <div className="modal-body"><p>请选择保存 {installTarget.name} 安装包的文件夹。</p></div>
            <div className="modal-footer">
              {btnCancel}
              <button className="primary-button" onClick={() => void pickDownloadPath()}>选择文件夹</button>
            </div>
          </div>
        </div>
      )
    }

    if (step === 'downloading') {
      return (
        <div className="modal-overlay">
          <div className="modal-dialog">
            <div className="modal-header"><h3>正在下载 {installTarget.name}</h3></div>
            <div className="modal-body"><div className="install-progress"><span className="spinner" />正在下载安装包…</div></div>
          </div>
        </div>
      )
    }

    if (step === 'select-install') {
      return (
        <div className="modal-overlay" onClick={closeInstall}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header"><h3>选择安装位置</h3>{btnClose}</div>
            <div className="modal-body"><p>下载完成。请选择安装 {installTarget.name} 的目标文件夹。</p></div>
            <div className="modal-footer">{btnCancel}</div>
          </div>
        </div>
      )
    }

    if (step === 'installing') {
      return (
        <div className="modal-overlay">
          <div className="modal-dialog">
            <div className="modal-header"><h3>正在安装 {installTarget.name}</h3></div>
            <div className="modal-body"><div className="install-progress"><span className="spinner" />正在静默安装…</div></div>
          </div>
        </div>
      )
    }

    if (step === 'done') {
      return (
        <div className="modal-overlay" onClick={closeInstall}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header"><h3>安装完成</h3>{btnClose}</div>
            <div className="modal-body"><div className="install-progress success">✓ {installTarget.name} 安装完成</div></div>
            <div className="modal-footer">{btnDone}</div>
          </div>
        </div>
      )
    }

    if (step === 'failed') {
      return (
        <div className="modal-overlay" onClick={closeInstall}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header"><h3>安装失败</h3>{btnClose}</div>
            <div className="modal-body"><div className="install-progress error">✗ {stepMessage}</div></div>
            <div className="modal-footer">{btnDone}</div>
          </div>
        </div>
      )
    }

    return null
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
          <div className="agent-list-panel panel"><div className="panel-heading"><div><h3>本机 Agent</h3><p>共 {agents.length} 个适配器 · Windows 11</p></div><button className="icon-button" onClick={() => void loadAgents()}>↻</button></div><div className="agent-list">{agents.map((agent) => <button key={agent.id} className={`agent-row ${selectedId === agent.id ? "selected" : ""}`} onClick={() => setSelectedId(agent.id)}><div className="agent-avatar"><img className="agent-icon-img" src={agentIcon[agent.id] ?? ""} alt={agent.name} onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; (e.target as HTMLImageElement).parentElement!.textContent = agent.name.slice(0, 1) }} /></div><div className="agent-row-copy"><div><strong>{agent.name}</strong>{agent.installed && <span className="dot-installed" title="已安装" />}</div><span>{agent.detail}</span></div><div className="agent-row-meta"><span className={statusClass(agent.status)}>{statusLabel[agent.status]}</span>{agent.running && <span className="running-dot">运行中</span>}</div></button>)}</div></div>

          {selected && <div className="detail-panel panel"><div className="panel-heading"><div className="detail-heading-with-icon"><img className="detail-icon-img" src={agentIcon[selected.id] ?? ""} alt={selected.name} onError={(e) => { (e.target as HTMLImageElement).style.display = "none" }} /><div><p className="eyebrow">AGENT DETAIL</p><h3>{selected.name}</h3></div></div><span className="publisher-chip">{selected.publisher}</span></div>{selected.installed ? <><div className="version-hero"><div><span>当前版本</span><strong>{selected.version?.product ?? '未识别'}</strong>{selected.version?.component && selected.version.component !== selected.version?.product && <small>组件 {selected.version.component}</small>}{selected.version?.pe && selected.version.pe !== selected.version?.product && <small>PE {selected.version.pe}</small>}</div><div className="version-arrow">→</div><div><span>最新版本</span><strong className={selected.latestVersion ? 'version-highlight' : ''}>{selected.latestVersion ?? '等待检查'}</strong><small>{selected.status === 'manual-action' ? '官方入口可更新' : '用户确认后更新'}</small></div></div><div className="detail-info"><div><span>安装位置</span><code>{selected.installPath ?? '未知'}</code></div><div><span>执行文件</span><code>{selected.executablePath ?? selected.installPath ?? '未发现'}</code></div><div><span>更新方式</span><strong>{selected.updateMode === 'manual-action' ? '官方入口 / 手动操作' : '内置更新器'}</strong></div><div><span>运行状态</span><strong>{selected.running ? '运行中' : '未运行'}</strong></div></div><div className="action-row"><button className="primary-button" onClick={() => void check(selected.id)} disabled={working === selected.id}>{working === selected.id ? '处理中…' : selected.latestVersion ? '检查更新' : '检查版本'}</button>{selected.status === 'manual-action' && <button className="secondary-button" onClick={() => void openOfficial(selected.id)}>打开官方入口</button>}{selected.status === 'update-available' && <button className="secondary-button" onClick={() => void update(selected.id)}>确认更新</button>}</div></> : <div className="empty-detail"><div className="empty-icon">＋</div><h4>尚未检测到 {selected.name}</h4><p>安装后重新扫描，管理器会自动识别版本和更新。</p><div className="action-row"><button className="primary-button" onClick={() => void requestInstall(selected)} disabled={working === selected.id}>{working === selected.id ? '安装中…' : '一键安装'}</button><button className="secondary-button" onClick={() => void openOfficial(selected.id)}>打开官方下载</button></div></div>}</div>}
        </section>
        <section className="notice"><span>ⓘ</span><p><strong>安全更新策略</strong>　每次更新都会先创建本机备份，并按照更新前运行状态决定是否重启。WorkBuddy 和 Marvis 的静默安装参数尚未在本机验证时，会提供官方入口而不是误报更新成功。</p></section>

        {renderDialog()}
      </main>
    </div>
  )
}

export default App
