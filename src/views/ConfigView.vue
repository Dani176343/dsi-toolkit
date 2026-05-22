<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { AppConfig, KnownRepo } from '../types'

const config = ref<AppConfig>({
  repos: [],
  known_repos: [],
  auto_pull_repos: [],
  git_credentials: null,
  maven_version: '3.9.6',
})

// login form — separado para não guardar token em texto visível antes de submeter
const loginHost     = ref('')
const loginUsername = ref('')
const loginToken    = ref('')
const loginTesting  = ref(false)
const loginResult   = ref<{ ok: boolean; text: string } | null>(null)

const isLoggedIn = computed(() => !!config.value.git_credentials)

function loadLoginForm() {
  const c = config.value.git_credentials
  if (c) {
    loginHost.value     = c.host
    loginUsername.value = c.username
    loginToken.value    = c.token
  }
}

function applyLogin() {
  if (!loginHost.value || !loginUsername.value || !loginToken.value) return
  config.value.git_credentials = {
    host:     loginHost.value.trim(),
    username: loginUsername.value.trim(),
    token:    loginToken.value.trim(),
  }
}

function clearLogin() {
  config.value.git_credentials = null
  loginHost.value = ''
  loginUsername.value = ''
  loginToken.value = ''
  loginResult.value = null
}

function openTokenPage() {
  const host = loginHost.value.trim() || config.value.git_credentials?.host
  if (!host) return
  // GitLab: /-/user_settings/personal_access_tokens
  // GitHub: /settings/tokens/new
  const url = host.includes('github.com')
    ? `https://github.com/settings/tokens/new?scopes=repo&description=dsi-toolkit`
    : `https://${host}/-/user_settings/personal_access_tokens?name=dsi-toolkit&scopes=read_repository,write_repository`
  openUrl(url)
}

async function testConnection() {
  applyLogin()
  await saveConfig()
  loginTesting.value = true
  loginResult.value = null
  try {
    const msg = await invoke<string>('test_git_connection')
    loginResult.value = { ok: true, text: msg }
  } catch (e) {
    loginResult.value = { ok: false, text: String(e) }
  } finally {
    loginTesting.value = false
  }
}

async function saveConfig() {
  await invoke('set_config', { config: config.value })
  saved.value = true
  setTimeout(() => { saved.value = false }, 2000)
}
const saved = ref(false)
const newAutoPull = ref('')
const detecting = ref(false)

async function autoDetect() {
  detecting.value = true
  try {
    config.value = await invoke<AppConfig>('detect_install_scripts')
  } finally {
    detecting.value = false
  }
}

onMounted(async () => {
  config.value = await invoke<AppConfig>('get_config')
  loadLoginForm()
})

// Novo repo conhecido
const newRepoName = ref('')
const newRepoFolders = ref('')   // CSV, ex: "projetos-sdk,projetos-sdk-master"

// Edição de variantes inline
const editingFolders = ref<Record<string, string>>({})

async function save() {
  applyLogin()
  await saveConfig()
}

function addKnownRepo() {
  const name = newRepoName.value.trim()
  if (!name) return
  if (config.value.known_repos.some(k => k.name === name)) return
  const folders = newRepoFolders.value
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)
  config.value.known_repos.push({
    name,
    folder_names: folders.length ? folders : [name],
  })
  newRepoName.value = ''
  newRepoFolders.value = ''
}

function removeKnownRepo(name: string) {
  config.value.known_repos = config.value.known_repos.filter(k => k.name !== name)
  config.value.auto_pull_repos = config.value.auto_pull_repos.filter(n => n !== name)
}

function startEditFolders(k: KnownRepo) {
  editingFolders.value[k.name] = k.folder_names.join(', ')
}

function commitEditFolders(k: KnownRepo) {
  const val = editingFolders.value[k.name] ?? ''
  const folders = val.split(',').map(s => s.trim()).filter(Boolean)
  k.folder_names = folders.length ? folders : [k.name]
  delete editingFolders.value[k.name]
}
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h2>Configuração</h2>
      <button class="btn-primary" @click="save">
        {{ saved ? 'Guardado!' : 'Guardar tudo' }}
      </button>
    </div>

    <!-- Login Git -->
    <section class="section login-section">
      <h3>Autenticação Git</h3>

      <div v-if="isLoggedIn" class="login-status login-status--ok">
        <span class="login-dot"></span>
        Autenticado como <strong>{{ config.git_credentials!.username }}</strong>
        em <strong>{{ config.git_credentials!.host }}</strong>
        <button class="btn-link" @click="clearLogin">Sair</button>
      </div>
      <div v-else class="login-status login-status--off">
        <span class="login-dot"></span>
        Sem sessão configurada
      </div>

      <div class="login-form">
        <label class="field">
          <span>Servidor (hostname)</span>
          <input v-model="loginHost" placeholder="gitlab.cma.pt" autocomplete="off" />
        </label>
        <label class="field">
          <span>Utilizador</span>
          <input v-model="loginUsername" placeholder="joao.silva" autocomplete="username" />
        </label>
        <label class="field">
          <span>
            Token / Password
            <button
              class="btn-token-link"
              :disabled="!loginHost && !config.git_credentials?.host"
              :title="!loginHost && !config.git_credentials?.host ? 'Preenche o servidor primeiro' : 'Abre o browser para criar um PAT'"
              @click.prevent="openTokenPage"
            >Obter token no GitLab →</button>
          </span>
          <input v-model="loginToken" type="password" placeholder="glpat-xxxxxxxxxxxx" autocomplete="current-password" />
          <span class="hint">Personal Access Token com scope <code>read_repository</code> (e <code>write_repository</code> para push).</span>
        </label>
        <div class="login-actions">
          <button class="btn-primary" :disabled="!loginHost || !loginUsername || !loginToken" @click="save">
            {{ saved ? 'Guardado!' : 'Guardar' }}
          </button>
          <button :disabled="loginTesting || !loginHost || !loginUsername || !loginToken" @click="testConnection">
            {{ loginTesting ? 'A testar…' : 'Testar ligação' }}
          </button>
        </div>
        <div v-if="loginResult" :class="['login-result', loginResult.ok ? 'login-result--ok' : 'login-result--err']">
          {{ loginResult.text }}
        </div>
      </div>
    </section>

    <!-- Repos conhecidos -->
    <section class="section">
      <h3>Repos conhecidos</h3>
      <p class="hint">
        Para cada repo define o <strong>nome canónico</strong> (apresentado na UI)
        e as <strong>variantes de pasta</strong> que o autodiscovery aceita.
      </p>

      <div class="known-list">
        <div v-for="k in config.known_repos" :key="k.name" class="known-row">
          <span class="known-name">{{ k.name }}</span>

          <!-- variantes em modo leitura -->
          <template v-if="!(k.name in editingFolders)">
            <div class="folder-tags">
              <span v-for="f in k.folder_names" :key="f" class="folder-tag">{{ f }}</span>
            </div>
            <button class="btn-icon" title="Editar variantes" @click="startEditFolders(k)">✏</button>
          </template>

          <!-- variantes em modo edição -->
          <template v-else>
            <input
              class="folders-input"
              v-model="editingFolders[k.name]"
              placeholder="pasta1, pasta2"
              @keydown.enter="commitEditFolders(k)"
              @blur="commitEditFolders(k)"
            />
            <span class="hint-inline">separado por vírgulas</span>
          </template>

          <button class="btn-icon btn-icon--danger" title="Remover" @click="removeKnownRepo(k.name)">×</button>
        </div>
        <div v-if="config.known_repos.length === 0" class="muted">Nenhum repo definido.</div>
      </div>

      <div class="add-row" style="margin-top:.75rem">
        <input v-model="newRepoName" placeholder="nome-canónico" @keydown.enter="addKnownRepo" style="max-width:180px" />
        <input v-model="newRepoFolders" placeholder="pasta1, pasta2 (opcional)" @keydown.enter="addKnownRepo" style="flex:1;max-width:300px" />
        <button @click="addKnownRepo">Adicionar</button>
      </div>
    </section>

    <!-- Pull automático -->
    <section class="section">
      <h3>Pull automático no arranque</h3>
      <p class="hint">Repos puxados automaticamente sempre que a app inicia.</p>
      <div class="tag-list">
        <span v-for="name in config.auto_pull_repos" :key="name" class="tag tag--auto">
          {{ name }}
          <button class="tag-remove" @click="config.auto_pull_repos = config.auto_pull_repos.filter(n => n !== name)">×</button>
        </span>
        <span v-if="config.auto_pull_repos.length === 0" class="muted">Nenhum repo em auto-pull.</span>
      </div>
      <div class="add-row">
        <select v-model="newAutoPull">
          <option value="" disabled>Escolhe um repo conhecido…</option>
          <option
            v-for="k in config.known_repos.filter(k => !config.auto_pull_repos.includes(k.name))"
            :key="k.name"
            :value="k.name"
          >{{ k.name }}</option>
        </select>
        <button :disabled="!newAutoPull" @click="() => { if (newAutoPull) { config.auto_pull_repos.push(newAutoPull); newAutoPull = '' } }">
          Adicionar
        </button>
      </div>
    </section>

    <!-- Scripts por repo -->
    <section class="section">
      <div class="section-header">
        <h3>installJars.sh por repositório</h3>
        <button :disabled="detecting" @click="autoDetect">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
          {{ detecting ? 'A procurar…' : 'Auto-detetar scripts' }}
        </button>
      </div>
      <p class="hint">Procura <code>installJars.sh</code> na raiz de cada repo. Podes também editar o path manualmente.</p>
      <div v-if="config.repos.length === 0" class="muted">Nenhum repo configurado (corre o Autodiscovery primeiro).</div>
      <div v-else class="script-list">
        <div v-for="r in config.repos" :key="r.name" class="script-row">
          <span class="script-repo-name">{{ r.name }}</span>
          <input
            v-model="r.install_script"
            :placeholder="`${r.path}\\installJars.sh`"
            class="script-input"
          />
          <span v-if="r.install_script" class="script-found">✓</span>
          <span v-else class="script-missing">—</span>
        </div>
      </div>
    </section>

    <!-- Maven -->
    <section class="section">
      <h3>Maven</h3>
      <label class="field">
        <span>Versão padrão</span>
        <input v-model="config.maven_version" placeholder="3.9.6" style="max-width:160px" />
      </label>
    </section>

    <!-- Repos resolvidos -->
    <section class="section">
      <h3>Repos configurados</h3>
      <p class="hint">Paths já resolvidos pelo autodiscovery. Gerir na tab Repositórios.</p>
      <div v-if="config.repos.length === 0" class="muted">Nenhum repo configurado ainda.</div>
      <div v-else class="repo-list">
        <div v-for="r in config.repos" :key="r.name" class="repo-row">
          <span class="repo-name">{{ r.name }}</span>
          <span class="repo-path">{{ r.path }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.view { padding: 1.5rem 2rem; }
.view-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem; }
.view-header h2 { margin: 0; }

.section { margin-bottom: 2rem; }
.section h3 { margin: 0 0 .5rem; font-size: .95rem; text-transform: uppercase; letter-spacing: .05em; color: #aaa; }
.hint { font-size: .82rem; color: #777; margin: 0 0 .75rem; }
.hint-inline { font-size: .75rem; color: #666; }
.muted { color: #666; font-size: .85rem; }

/* known repos */
.known-list { display: flex; flex-direction: column; gap: .4rem; }
.known-row {
  display: flex; align-items: center; gap: .6rem; flex-wrap: wrap;
  background: #1a1a1a; border: 1px solid #2e2e2e; border-radius: 6px;
  padding: .4rem .75rem;
}
.known-name { font-weight: 600; min-width: 160px; font-size: .9rem; }
.folder-tags { display: flex; flex-wrap: wrap; gap: .3rem; flex: 1; }
.folder-tag {
  background: #222; border: 1px solid #3a3a3a; border-radius: 4px;
  padding: .1em .5em; font-family: monospace; font-size: .78rem; color: #aaa;
}
.folders-input { flex: 1; min-width: 200px; font-family: monospace; font-size: .82rem; }
.btn-icon {
  background: none; border: none; color: #666; cursor: pointer;
  font-size: 1rem; padding: .1em .3em; border-radius: 4px;
}
.btn-icon:hover { color: #ccc; background: #2a2a2a; }
.btn-icon--danger:hover { color: #f06060; }

/* tags */
.tag-list { display: flex; flex-wrap: wrap; gap: .4rem; margin-bottom: .75rem; min-height: 2rem; align-items: center; }
.tag { background: #1e3a5a; color: #7ec8f0; padding: .25em .6em .25em .8em; border-radius: 12px; font-size: .85rem; display: flex; align-items: center; gap: .3rem; }
.tag--auto { background: #1a2e10; color: #7fcf70; }
.tag-remove { background: none; border: none; color: inherit; cursor: pointer; padding: 0; font-size: 1rem; line-height: 1; opacity: .7; }
.tag-remove:hover { opacity: 1; }

select { background: #0f0f0f; border: 1px solid #3a3a3a; border-radius: 6px; color: #e0e0e0; padding: .45em .75em; font-size: .9rem; font-family: inherit; outline: none; }

.add-row { display: flex; gap: .5rem; flex-wrap: wrap; }

.field { display: flex; flex-direction: column; gap: .35rem; margin-bottom: .75rem; }
.field span { font-size: .85rem; color: #ccc; }
.field input { max-width: 560px; }

.repo-list { display: flex; flex-direction: column; gap: .4rem; }
.repo-row { background: #1e1e1e; border: 1px solid #333; border-radius: 6px; padding: .5rem .75rem; display: flex; gap: 1rem; align-items: baseline; }
.repo-name { font-weight: 600; min-width: 160px; }
.repo-path { font-family: monospace; font-size: .8rem; color: #888; }

.section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: .5rem; }
.section-header h3 { margin: 0; }

.script-list { display: flex; flex-direction: column; gap: .4rem; }
.script-row { display: flex; align-items: center; gap: .75rem; }
.script-repo-name { font-weight: 600; min-width: 150px; font-size: .875rem; }
.script-input { flex: 1; font-family: 'Consolas', monospace; font-size: .8rem; }
.script-found  { color: #5aab5a; font-size: .85rem; flex-shrink: 0; }
.script-missing { color: #444; font-size: .85rem; flex-shrink: 0; }

.btn-primary { background: #1a4a8a; border-color: #2060b0; color: #fff; }
.btn-primary:hover { background: #1e5aa0; }
.btn-link { background: none; border: none; color: #7ec8f0; cursor: pointer; padding: 0 .25em; font-size: inherit; text-decoration: underline; }
.btn-token-link { background: none; border: none; color: #7ec8f0; cursor: pointer; padding: 0; font-size: .8rem; text-decoration: underline; margin-left: .5rem; }
.btn-token-link:disabled { color: #444; cursor: not-allowed; text-decoration: none; }

/* login */
.login-section { background: #161e2a; border: 1px solid #1e3050; border-radius: 10px; padding: 1rem 1.25rem; }
.login-status { display: flex; align-items: center; gap: .5rem; font-size: .88rem; margin-bottom: 1rem; flex-wrap: wrap; }
.login-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.login-status--ok  .login-dot { background: #6fcf6f; }
.login-status--off .login-dot { background: #666; }
.login-status--ok  { color: #ccc; }
.login-status--off { color: #666; }
.login-form { display: flex; flex-direction: column; gap: .5rem; }
.login-actions { display: flex; gap: .5rem; margin-top: .25rem; }
.login-result { margin-top: .5rem; padding: .4rem .75rem; border-radius: 6px; font-size: .85rem; }
.login-result--ok  { background: #1a3d1a; color: #6fcf6f; }
.login-result--err { background: #3d1010; color: #f06060; }
</style>
