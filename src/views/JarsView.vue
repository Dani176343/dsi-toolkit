<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AppConfig, JarInfo, RepoConfig } from '../types'

const config = ref<AppConfig | null>(null)
const jarsMap = ref<Record<string, JarInfo[]>>({})
const loadingMap = ref<Record<string, boolean>>({})
const detecting = ref(false)

// Estado de instalação por repo
interface InstallState {
  running: boolean
  installed: number
  total: number
  lastJar: string
  incremental: boolean
  result: null | { ok: boolean; count: number; error?: string }
}
const installState = ref<Record<string, InstallState>>({})

const activeRepos = computed(() =>
  (config.value?.repos ?? []).filter(r => r.install_script || r.jars_dir)
)
// Só avisa se a deteção nunca foi corrida (nenhum repo tem install_script).
// Repos como environmentcma legitimamente não têm JARs — não devem disparar o aviso.
const needsDetection = computed(() => {
  const repos = config.value?.repos ?? []
  if (repos.length === 0) return false
  return repos.every(r => !r.install_script)
})

let unlistenProgress: UnlistenFn | null = null
let unlistenDone: UnlistenFn | null = null

onMounted(async () => {
  config.value = await invoke<AppConfig>('get_config')
  await loadAllJars()

  unlistenProgress = await listen<{ repo: string; installed: number; total: number; jar: string }>(
    'install-progress',
    ({ payload }) => {
      if (!installState.value[payload.repo]) return
      installState.value[payload.repo].installed = payload.installed
      installState.value[payload.repo].total = payload.total
      installState.value[payload.repo].lastJar = payload.jar
    }
  )

  unlistenDone = await listen<{ repo: string; installed: number; total: number }>(
    'install-done',
    async ({ payload }) => {
      if (!installState.value[payload.repo]) return
      installState.value[payload.repo].running = false
      installState.value[payload.repo].result = { ok: true, count: payload.installed }
      // Recarrega lista para actualizar timestamps
      const repo = config.value?.repos.find(r => r.name === payload.repo)
      if (repo) await loadJarsFor(repo)
    }
  )
})

onUnmounted(() => {
  unlistenProgress?.()
  unlistenDone?.()
})

async function autoDetect() {
  detecting.value = true
  try {
    config.value = await invoke<AppConfig>('detect_install_scripts')
    await loadAllJars()
  } finally {
    detecting.value = false
  }
}

async function loadAllJars() {
  if (!config.value) return
  await Promise.all(config.value.repos.filter(r => r.jars_dir).map(r => loadJarsFor(r)))
}

async function loadJarsFor(repo: RepoConfig) {
  if (!repo.jars_dir) return
  loadingMap.value[repo.name] = true
  try {
    jarsMap.value[repo.name] = await invoke<JarInfo[]>('list_jars', { jarsDir: repo.jars_dir })
  } catch { jarsMap.value[repo.name] = [] }
  finally { loadingMap.value[repo.name] = false }
}

async function runInstall(repo: RepoConfig) {
  if (!repo.jars_dir || !repo.install_script) return

  const summary = repoSummary(repo.name)
  const needsUpdate = summary.outdated + summary.missing
  // Usa -t (incremental) se menos de 50% precisam de ser instalados
  const incremental = needsUpdate > 0 && needsUpdate < summary.total * 0.5

  const total = jarsMap.value[repo.name]?.length ?? 0
  installState.value[repo.name] = { running: true, installed: 0, total, lastJar: '', result: null, incremental }

  try {
    const count = await invoke<number>('run_install_jars', {
      repoName: repo.name,
      jarsDir: repo.jars_dir,
      scriptPath: repo.install_script,
      incremental,
    })
    // install-done event will update state — fallback if somehow missed
    if (installState.value[repo.name]?.running) {
      installState.value[repo.name].running = false
      installState.value[repo.name].result = { ok: true, count }
      await loadJarsFor(repo)
    }
  } catch (e) {
    installState.value[repo.name].running = false
    installState.value[repo.name].result = { ok: false, count: 0, error: String(e) }
  }
}

// ── helpers ──────────────────────────────────────────────────────────────────

function progressPct(state: InstallState): number {
  if (!state.total) return 0
  return Math.round((state.installed / state.total) * 100)
}

function formatBytes(b: number): string {
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${(b / 1024 / 1024).toFixed(1)} MB`
}

function formatTs(ts: number | null): string {
  if (ts === null) return '—'
  return new Date(ts * 1000).toLocaleString('pt-PT', { dateStyle: 'short', timeStyle: 'short' })
}

type S = 'installed' | 'outdated' | 'missing'
function jarState(jar: JarInfo): S {
  if (jar.install_timestamp === null) return 'missing'
  if (jar.last_modified !== null && jar.last_modified > jar.install_timestamp) return 'outdated'
  return 'installed'
}

function repoSummary(repoName: string) {
  const list = jarsMap.value[repoName] ?? []
  return {
    total:     list.length,
    installed: list.filter(j => jarState(j) === 'installed').length,
    outdated:  list.filter(j => jarState(j) === 'outdated').length,
    missing:   list.filter(j => jarState(j) === 'missing').length,
  }
}
</script>

<template>
  <div class="view">
    <div class="page-header">
      <h2>JARs Maven</h2>
      <button :disabled="detecting" @click="autoDetect">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
        {{ detecting ? 'A detetar…' : 'Auto-detetar' }}
      </button>
    </div>

    <div v-if="activeRepos.length === 0" class="empty-state">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="1.5"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/></svg>
      <p>Nenhum diretório de JARs encontrado.</p>
      <p>Clica <strong>Auto-detetar</strong>.</p>
    </div>

    <template v-else>
      <div v-if="needsDetection" class="hint-bar">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#c09020" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        Alguns repos não foram analisados.
        <button @click="autoDetect">Auto-detetar agora</button>
      </div>

      <div v-for="repo in activeRepos" :key="repo.name" class="repo-section">
        <!-- Header do repo -->
        <div class="repo-section-header">
          <div class="repo-section-title">
            <span class="repo-name">{{ repo.name }}</span>
            <span class="repo-jars-path">{{ repo.jars_dir ?? '—' }}</span>
          </div>
          <div class="repo-section-actions">
            <template v-if="repo.jars_dir && !installState[repo.name]?.running">
              <span v-if="repoSummary(repo.name).outdated" class="pill warn">
                ↻ {{ repoSummary(repo.name).outdated }} desatualizado{{ repoSummary(repo.name).outdated > 1 ? 's' : '' }}
              </span>
              <span v-else-if="repoSummary(repo.name).total" class="pill ok">
                ✓ {{ repoSummary(repo.name).installed }}/{{ repoSummary(repo.name).total }}
              </span>
            </template>
            <button
              v-if="repo.install_script"
              class="btn-primary"
              :disabled="installState[repo.name]?.running"
              @click="runInstall(repo)"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="5 3 19 12 5 21 5 3"/></svg>
              {{ installState[repo.name]?.running ? 'A instalar…' : 'Instalar JARs' }}
            </button>
            <button :disabled="loadingMap[repo.name]" @click="loadJarsFor(repo)" title="Recarregar lista">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
            </button>
          </div>
        </div>

        <!-- Progress bar (só durante instalação) -->
        <div v-if="installState[repo.name]?.running" class="progress-area">
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: progressPct(installState[repo.name]) + '%' }"></div>
          </div>
          <div class="progress-meta">
            <span class="progress-jar">
              <span v-if="installState[repo.name].incremental" class="badge-incremental">⚡ modo rápido</span>
              {{ installState[repo.name].lastJar || '…' }}
            </span>
            <span class="progress-count">{{ installState[repo.name].installed }} / {{ installState[repo.name].total }}</span>
          </div>
        </div>

        <!-- Resultado (só erro ou mensagem de sucesso) -->
        <div
          v-if="installState[repo.name]?.result && !installState[repo.name].running"
          :class="['install-result', installState[repo.name].result!.ok ? 'ok' : 'err']"
        >
          <template v-if="installState[repo.name].result!.ok">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
            {{ installState[repo.name].result!.count }} JARs instalados com sucesso
          </template>
          <template v-else>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            <pre>{{ installState[repo.name].result!.error }}</pre>
          </template>
        </div>

        <!-- Sem script -->
        <div v-if="!repo.install_script" class="no-script">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#555" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
          installJars.sh não encontrado neste repo.
        </div>

        <!-- Tabela de JARs -->
        <div v-if="loadingMap[repo.name]" class="loading-row">A carregar…</div>
        <div v-else-if="!repo.jars_dir" class="loading-row muted">Diretório de JARs não detetado.</div>
        <div v-else-if="!jarsMap[repo.name]?.length" class="loading-row muted">Nenhum .jar encontrado.</div>
        <table v-else class="jar-table">
          <thead>
            <tr>
              <th></th>
              <th>Nome</th>
              <th>Tamanho</th>
              <th>Modificado</th>
              <th>Instalado</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="jar in jarsMap[repo.name]" :key="jar.name" :class="['jar-row', jarState(jar)]">
              <td><span :class="['dot', jarState(jar)]"></span></td>
              <td class="jar-name">{{ jar.name }}</td>
              <td class="mono">{{ formatBytes(jar.size_bytes) }}</td>
              <td class="mono">{{ formatTs(jar.last_modified) }}</td>
              <td class="mono">{{ formatTs(jar.install_timestamp) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<style scoped>
.view { padding: 1.5rem 2rem; }
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.25rem; }
.page-header h2 { margin: 0; }

.empty-state { text-align: center; color: #444; margin-top: 5rem; display: flex; flex-direction: column; align-items: center; gap: .5rem; }
.empty-state p { margin: 0; font-size: .875rem; }

.hint-bar { display: flex; align-items: center; gap: .5rem; background: #1e1600; border: 1px solid #3d2e00; border-radius: 7px; padding: .5rem .875rem; font-size: .8rem; color: #c09020; margin-bottom: 1rem; }
.hint-bar button { padding: .25em .65em; font-size: .78rem; }

/* sections */
.repo-section { background: #161616; border: 1px solid #222; border-radius: 8px; margin-bottom: .75rem; overflow: hidden; }
.repo-section-header { display: flex; align-items: center; justify-content: space-between; padding: .75rem 1rem; border-bottom: 1px solid #1e1e1e; gap: 1rem; }
.repo-section-title { display: flex; flex-direction: column; gap: .15rem; min-width: 0; }
.repo-name { font-weight: 600; font-size: .95rem; color: #e0e0e0; }
.repo-jars-path { font-family: 'Consolas', monospace; font-size: .72rem; color: #444; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.repo-section-actions { display: flex; align-items: center; gap: .4rem; flex-shrink: 0; }

/* progress */
.progress-area { padding: .75rem 1rem; border-bottom: 1px solid #1e1e1e; }
.progress-track { height: 4px; background: #222; border-radius: 2px; overflow: hidden; margin-bottom: .4rem; }
.progress-fill { height: 100%; background: #2a6bd4; border-radius: 2px; transition: width .3s ease; }
.progress-meta { display: flex; justify-content: space-between; font-size: .72rem; color: #555; }
.progress-jar { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 70%; font-family: 'Consolas', monospace; display: flex; align-items: center; gap: .4rem; }
.badge-incremental { background: #1a1000; color: #c09020; border: 1px solid #3d2800; border-radius: 4px; padding: .1em .4em; font-size: .7rem; font-family: sans-serif; flex-shrink: 0; }
.progress-count { flex-shrink: 0; }

/* result */
.install-result { display: flex; align-items: flex-start; gap: .4rem; padding: .5rem 1rem; font-size: .82rem; border-bottom: 1px solid #1e1e1e; }
.install-result.ok { background: #0a1e0a; color: #5aab5a; }
.install-result.err { background: #1e0808; color: #c05050; }
.install-result pre { margin: 0; white-space: pre-wrap; word-break: break-all; font-family: 'Consolas', monospace; font-size: .78rem; }

.no-script { display: flex; align-items: center; gap: .4rem; font-size: .78rem; color: #555; padding: .5rem 1rem; border-bottom: 1px solid #1e1e1e; }
.loading-row { padding: .75rem 1rem; font-size: .82rem; color: #555; }
.muted { color: #444; }

/* pills */
.pill { padding: .18em .6em; border-radius: 4px; font-size: .73rem; font-weight: 600; }
.pill.ok   { background: #0d2010; color: #5aab5a; }
.pill.warn { background: #201400; color: #c09020; }

/* table */
.jar-table { width: 100%; border-collapse: collapse; font-size: .8rem; }
.jar-table th { text-align: left; padding: .35rem 1rem; border-bottom: 1px solid #1e1e1e; color: #444; font-weight: 500; font-size: .7rem; text-transform: uppercase; letter-spacing: .05em; }
.jar-table td { padding: .35rem 1rem; border-bottom: 1px solid #181818; }
.jar-table tbody tr:last-child td { border-bottom: none; }
.jar-table tbody tr:hover td { background: #1a1a1a; }
.jar-row.outdated td { background: #141000; }

.jar-name { font-family: 'Consolas', monospace; color: #c0c0c0; }
.mono { font-family: 'Consolas', monospace; color: #555; }

.dot { display: inline-block; width: 7px; height: 7px; border-radius: 50%; }
.dot.installed { background: #3a8a3a; }
.dot.outdated  { background: #9a7020; }
.dot.missing   { background: #2e2e2e; border: 1px solid #444; }
</style>
