<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

const state = ref<'idle' | 'checking' | 'available' | 'downloading' | 'done' | 'error'>('idle')
const newVersion = ref('')
const progress = ref(0)
const errorMsg = ref('')

let checkInterval: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  // Aguarda 3s para não atrasar o arranque
  await new Promise(r => setTimeout(r, 3000))
  await checkUpdate()
  // Verifica novamente a cada hora
  checkInterval = setInterval(checkUpdate, 60 * 60 * 1000)
})

onUnmounted(() => {
  if (checkInterval) clearInterval(checkInterval)
})

async function checkUpdate() {
  state.value = 'checking'
  try {
    const update = await check()
    if (update?.available) {
      newVersion.value = update.version
      state.value = 'available'
    } else {
      state.value = 'idle'
    }
  } catch {
    state.value = 'idle' // falha silenciosa — não incomoda o utilizador
  }
}

async function install() {
  state.value = 'downloading'
  progress.value = 0
  try {
    const update = await check()
    if (!update?.available) return
    let totalSize = 0
    let downloaded = 0
    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        totalSize = event.data.contentLength ?? 0
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength
        if (totalSize > 0) progress.value = Math.round((downloaded / totalSize) * 100)
      }
    })
    state.value = 'done'
  } catch (e) {
    errorMsg.value = String(e)
    state.value = 'error'
  }
}

async function restartNow() {
  await relaunch()
}

function dismiss() {
  state.value = 'idle'
}
</script>

<template>
  <transition name="banner">
    <div v-if="state === 'available'" class="update-banner">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="8 17 12 21 16 17"/><line x1="12" y1="21" x2="12" y2="3"/>
      </svg>
      <span>Nova versão disponível: <strong>{{ newVersion }}</strong></span>
      <button class="btn-update" @click="install">Atualizar agora</button>
      <button class="btn-dismiss" @click="dismiss" title="Ignorar">✕</button>
    </div>

    <div v-else-if="state === 'downloading'" class="update-banner downloading">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="spin">
        <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
      </svg>
      <span>A transferir atualização… {{ progress }}%</span>
      <div class="mini-progress"><div class="mini-fill" :style="{ width: progress + '%' }"></div></div>
    </div>

    <div v-else-if="state === 'done'" class="update-banner done">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
      <span>Atualização instalada. Reinicia para aplicar.</span>
      <button class="btn-update" @click="restartNow">Reiniciar agora</button>
    </div>

    <div v-else-if="state === 'error'" class="update-banner error">
      <span>Erro ao atualizar: {{ errorMsg }}</span>
      <button class="btn-dismiss" @click="dismiss">✕</button>
    </div>
  </transition>
</template>

<style scoped>
.update-banner {
  position: fixed;
  bottom: 1.25rem;
  right: 1.25rem;
  display: flex;
  align-items: center;
  gap: .6rem;
  background: #0d2040;
  border: 1px solid #1e3a60;
  border-radius: 8px;
  padding: .6rem 1rem;
  font-size: .82rem;
  color: #90b8e0;
  z-index: 9999;
  box-shadow: 0 4px 20px rgba(0,0,0,.5);
  max-width: 420px;
}
.update-banner.downloading { background: #111; border-color: #2a2a2a; color: #888; }
.update-banner.done        { background: #0a1e0a; border-color: #1a3d1a; color: #5aab5a; }
.update-banner.error       { background: #1e0808; border-color: #3d1010; color: #c05050; }

.btn-update  { padding: .25em .75em; background: #1e4a8a; color: #c0d8f0; border: 1px solid #2a5a9a; border-radius: 5px; font-size: .78rem; cursor: pointer; white-space: nowrap; }
.btn-update:hover { background: #2a5a9a; }
.btn-dismiss { background: none; border: none; color: #555; cursor: pointer; font-size: .85rem; padding: 0 .2rem; line-height: 1; }
.btn-dismiss:hover { color: #888; }

.mini-progress { flex: 1; height: 3px; background: #222; border-radius: 2px; min-width: 80px; }
.mini-fill     { height: 100%; background: #2a6bd4; border-radius: 2px; transition: width .3s; }

.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.banner-enter-active, .banner-leave-active { transition: opacity .3s, transform .3s; }
.banner-enter-from, .banner-leave-to { opacity: 0; transform: translateY(8px); }
</style>
