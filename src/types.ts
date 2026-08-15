export type AgentStatus = 'not-installed' | 'checking' | 'up-to-date' | 'update-available' | 'manual-action' | 'error'
export type UpdateMode = 'native-updater' | 'staged-installer' | 'manual-action'

export interface VersionSnapshot {
  product: string | null
  component: string | null
  bootstrap: string | null
  channel: string | null
  pe: string | null
}

export interface AgentInstance {
  id: string
  name: string
  publisher: string
  installed: boolean
  installPath: string | null
  executablePath: string | null
  version: VersionSnapshot | null
  latestVersion: string | null
  status: AgentStatus
  running: boolean
  updateMode: UpdateMode
  officialUrl: string
  detail: string
  lastChecked: string | null
}

export interface UpdateResult {
  success: boolean
  message: string
  mode: UpdateMode
  officialUrl: string | null
  needsRestart: boolean
  previousVersion: string | null
  currentVersion: string | null
}



export interface ScheduleConfig {
  enabled: boolean
  time: string
  agentIds: string[]
}

