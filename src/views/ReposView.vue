<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AppConfig, RepoConfig, RepoStatus, DiscoveryResult } from '../types'

const config = ref<AppConfig | null>(null)
const statuses = ref<Record<string, RepoStatus>>({})
const pulling = ref<Record<string, boolean>>({})
const pullMsg = ref<Record<string, { ok: boolean; text: string }>>({})
const discovering = ref(false)
const refreshingAll = ref(false)
const pullingAll = ref(false)
const missingRepos = ref<string[]>([])
const manualPaths = ref<Record<string, string>>({})
const pullMsgTimers = ref<Record<string, ReturnType<typeof setTimeout>>>({})

let unlisten: UnlistenFn | null = null
let refreshInterval: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  config.value = await invoke<AppConfig>('get_config')

  unlisten = await listen<string[]>('repos-not-found', (e) => {
    missingRepos.value = e.payload
    e.payload.forEach(n => { manualPaths.value[n] = '' })
  })

  if (config.value.repos.length === 0 && config.value.known_repos.length > 0) {
    await runDiscovery()
  } else {
    await refreshAllStatuses()
  }
  await autoPullOnStart()

  // Verifica atualizações a cada minuto
  refreshInterval = setInterval(() => {
    if (!refreshingAll.value && !pullingAll.value) refreshAllStatuses()
  }, 60_000)
})

onUnmounted(() => {
  unlisten?.()
  if (refreshInterval) clearInterval(refreshInterval)
  Object.values(pullMsgTimers.value).forEach(clearTimeout)
})

async function runDiscovery() {
  discovering.value = true
  try {
    const result = await invoke<DiscoveryResult>('autodiscover_repos')
    config.value = await invoke<AppConfig>('get_config')
    await refreshAllStatuses()
    if (result.missing.length === 0) missingRepos.value = []
  } finally {
    discovering.value = false
  }
}

async function refreshAllStatuses() {
  if (!config.value) return
  refreshingAll.value = true
  await Promise.all(config.value.repos.map(r => fetchStatus(r)))
  refreshingAll.value = false
}

async function fetchStatus(repo: RepoConfig) {
  const status = await invoke<RepoStatus>('get_repo_status', { name: repo.name, path: repo.path })
  statuses.value[repo.name] = status
}

function setPullMsg(name: string, msg: { ok: boolean; text: string }) {
  // Cancela timer anterior caso exista
  if (pullMsgTimers.value[name]) clearTimeout(pullMsgTimers.value[name])
  pullMsg.value[name] = msg
  pullMsgTimers.value[name] = setTimeout(() => {
    delete pullMsg.value[name]
    delete pullMsgTimers.value[name]
  }, 5_000)
}

async function pull(repo: RepoConfig) {
  pulling.value[repo.name] = true
  try {
    const msg = await invoke<string>('git_pull', { name: repo.name, path: repo.path })
    setPullMsg(repo.name, { ok: true, text: msg })
    await fetchStatus(repo)
  } catch (e) {
    setPullMsg(repo.name, { ok: false, text: String(e) })
  } finally {
    pulling.value[repo.name] = false
  }
}

async function pullAll() {
  if (!config.value) return
  pullingAll.value = true
  await Promise.all(config.value.repos.map(r => pull(r)))
  pullingAll.value = false
}

async function autoPullOnStart() {
  if (!config.value) return
  const targets = config.value.repos.filter(r => config.value!.auto_pull_repos.includes(r.name))
  await Promise.all(targets.map(r => pull(r)))
}

async function submitManualPath(repoName: string) {
  const path = manualPaths.value[repoName]?.trim()
  if (!path) return
  config.value = await invoke<AppConfig>('add_repo', { name: repoName, path })
  missingRepos.value = missingRepos.value.filter(n => n !== repoName)
  const repo = config.value.repos.find(r => r.name === repoName)
  if (repo) await fetchStatus(repo)
}

async function removeRepo(name: string) {
  config.value = await invoke<AppConfig>('remove_repo', { name })
  delete statuses.value[name]
}

function statusLabel(name: string): string {
  const s = statuses.value[name]?.status
  if (!s) return '…'
  switch (s.type) {
    case 'upToDate': return 'Atualizado'
    case 'behind':   return `↓ ${s.commits} commits`
    case 'ahead':    return `↑ ${s.commits} commits`
    case 'diverged': return `↓${s.behind} ↑${s.ahead}`
    case 'noRemote': return 'Sem remoto'
    case 'error':    return 'Erro'
    default:         return '…'
  }
}

function statusClass(name: string): string {
  const s = statuses.value[name]?.status
  if (!s) return 'badge--loading'
  switch (s.type) {
    case 'upToDate': return 'badge--ok'
    case 'behind':   return 'badge--warn'
    case 'ahead':    return 'badge--info'
    case 'diverged': return 'badge--error'
    case 'noRemote': return 'badge--muted'
    case 'error':    return 'badge--error'
    default:         return 'badge--loading'
  }
}

function errorDetail(name: string): string {
  const s = statuses.value[name]?.status
  if (s?.type === 'error') return s.message
  return ''
}
</script>

<template>
  <div class="view">
    <div class="page-header">
      <h2>Repositórios</h2>
      <div class="header-actions">
        <button :disabled="discovering || refreshingAll" @click="runDiscovery">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
          {{ discovering ? 'A procurar…' : 'Autodiscovery' }}
        </button>
        <button :disabled="refreshingAll" @click="refreshAllStatuses">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
          {{ refreshingAll ? 'A verificar…' : 'Atualizar' }}
        </button>
        <button
          class="btn-primary"
          :disabled="pullingAll || !config?.repos.length"
          @click="pullAll"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="8 17 12 21 16 17"/><line x1="12" y1="21" x2="12" y2="3"/></svg>
          {{ pullingAll ? 'A puxar…' : 'Pull All' }}
        </button>
      </div>
    </div>

    <!-- Repos não encontrados -->
    <div v-if="missingRepos.length" class="missing-card">
      <div class="missing-title">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#f0c040" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        Repos não encontrados automaticamente
      </div>
      <div v-for="name in missingRepos" :key="name" class="missing-row">
        <span class="missing-name">{{ name }}</span>
        <input v-model="manualPaths[name]" :placeholder="`C:\\dev\\${name}`" class="missing-input" />
        <button @click="submitManualPath(name)">Adicionar</button>
      </div>
    </div>

    <div v-if="!config || config.repos.length === 0" class="empty-state">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="1.5"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2a2 2 0 01-2 2H8a2 2 0 01-2-2V9"/><path d="M12 12v3"/></svg>
      <p>Nenhum repositório configurado.</p>
      <p>Adiciona repos em <strong>Configuração</strong> e clica <strong>Autodiscovery</strong>.</p>
    </div>

    <div v-else class="repo-list">
      <div v-for="repo in config.repos" :key="repo.name" class="repo-card">
        <div class="repo-main">
          <div class="repo-left">
            <div class="repo-title-row">
              <span class="repo-name">{{ repo.name }}</span>
              <span v-if="config?.auto_pull_repos.includes(repo.name)" class="chip chip--auto">auto-pull</span>
            </div>
            <span class="repo-path">{{ repo.path }}</span>
          </div>
          <div class="repo-right">
            <span :class="['badge', statusClass(repo.name)]" :title="errorDetail(repo.name)">
              {{ statusLabel(repo.name) }}
            </span>
            <button
              class="btn-primary"
              :disabled="pulling[repo.name]"
              @click="pull(repo)"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
              {{ pulling[repo.name] ? 'A puxar…' : 'Pull' }}
            </button>
            <button class="btn-danger" title="Remover" @click="removeRepo(repo.name)">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          </div>
        </div>
        <div v-if="pullMsg[repo.name]?.text" :class="['pull-msg', pullMsg[repo.name].ok ? 'ok' : 'err']">
          {{ pullMsg[repo.name].text }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view { padding: 1.5rem 2rem; }

.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.25rem; }
.page-header h2 { margin: 0; }
.header-actions { display: flex; gap: .5rem; }

.missing-card { background: #1e1800; border: 1px solid #4a3800; border-radius: 8px; padding: .875rem 1rem; margin-bottom: 1rem; }
.missing-title { display: flex; align-items: center; gap: .4rem; font-weight: 600; font-size: .82rem; color: #f0c040; margin-bottom: .75rem; }
.missing-row { display: flex; align-items: center; gap: .5rem; margin-top: .4rem; }
.missing-name { font-weight: 600; min-width: 160px; font-size: .82rem; }
.missing-input { flex: 1; }

.empty-state { text-align: center; color: #444; margin-top: 5rem; display: flex; flex-direction: column; align-items: center; gap: .5rem; }
.empty-state p { margin: 0; font-size: .875rem; }

.repo-list { display: flex; flex-direction: column; gap: .5rem; }

.repo-card {
  background: #161616;
  border: 1px solid #222;
  border-radius: 8px;
  padding: .875rem 1rem;
  transition: border-color .15s;
}
.repo-card:hover { border-color: #2e2e2e; }

.repo-main { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
.repo-left { display: flex; flex-direction: column; gap: .25rem; min-width: 0; }
.repo-title-row { display: flex; align-items: center; gap: .5rem; }
.repo-name { font-weight: 600; font-size: .95rem; color: #e0e0e0; }
.repo-path { font-family: 'Consolas', monospace; font-size: .75rem; color: #555; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 60vw; }
.repo-right { display: flex; align-items: center; gap: .4rem; flex-shrink: 0; }

.chip { padding: .15em .55em; border-radius: 4px; font-size: .68rem; font-weight: 700; letter-spacing: .03em; }
.chip--auto { background: #0d2040; color: #6090d0; border: 1px solid #1e3a60; }

.badge { padding: .2em .65em; border-radius: 5px; font-size: .75rem; font-weight: 600; }
.badge--ok      { background: #0d2010; color: #5aab5a; border: 1px solid #1a3d1a; }
.badge--warn    { background: #201400; color: #c09020; border: 1px solid #3d2800; }
.badge--info    { background: #001830; color: #5090c0; border: 1px solid #0d2840; }
.badge--error   { background: #200808; color: #c05050; border: 1px solid #3d1010; }
.badge--muted   { background: #181818; color: #555; border: 1px solid #252525; }
.badge--loading { background: #181818; color: #666; border: 1px solid #252525; }

.pull-msg { margin-top: .6rem; font-size: .78rem; padding: .35em .65em; border-radius: 5px; }
.pull-msg.ok { background: #0d2010; color: #5aab5a; }
.pull-msg.err { background: #200808; color: #c05050; }
</style>
