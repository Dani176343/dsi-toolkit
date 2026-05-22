<script setup lang="ts">
import { ref } from 'vue'
import ReposView from './views/ReposView.vue'
import JarsView from './views/JarsView.vue'
import ConfigView from './views/ConfigView.vue'
import UpdateBanner from './components/UpdateBanner.vue'

type Tab = 'repos' | 'jars' | 'config'
const activeTab = ref<Tab>('repos')
</script>

<template>
  <UpdateBanner />
  <div class="app">
    <nav class="sidebar">
      <div class="brand">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>
        <span>DSI Toolkit</span>
      </div>

      <div class="nav-group">
        <button :class="['nav-item', activeTab === 'repos' && 'active']" @click="activeTab = 'repos'">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2a2 2 0 01-2 2H8a2 2 0 01-2-2V9"/><path d="M12 12v3"/></svg>
          Repositórios
        </button>
        <button :class="['nav-item', activeTab === 'jars' && 'active']" @click="activeTab = 'jars'">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/></svg>
          JARs Maven
        </button>
        <button :class="['nav-item', activeTab === 'config' && 'active']" @click="activeTab = 'config'">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
          Configuração
        </button>
      </div>

      <div class="sidebar-footer">CMA Abrantes · DSI</div>
    </nav>

    <main class="content">
      <ReposView v-if="activeTab === 'repos'" />
      <JarsView  v-if="activeTab === 'jars'" />
      <ConfigView v-if="activeTab === 'config'" />
    </main>
  </div>
</template>

<style>
:root {
  font-family: Inter, 'Segoe UI', Arial, sans-serif;
  font-size: 13px;
  line-height: 1.5;
  color: #d4d4d4;
  background: #111;
  -webkit-font-smoothing: antialiased;
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { width: 100%; height: 100%; overflow: hidden; }
#app { width: 100%; height: 100vh; display: flex; }

h2 { font-size: 1.15rem; font-weight: 600; color: #f0f0f0; }
h3 { font-size: .78rem; font-weight: 600; text-transform: uppercase; letter-spacing: .07em; color: #666; margin: 0 0 .75rem; }

input, select {
  background: #1a1a1a;
  border: 1px solid #2e2e2e;
  border-radius: 6px;
  color: #d4d4d4;
  padding: .45em .75em;
  font-size: .875rem;
  font-family: inherit;
  outline: none;
  transition: border-color .15s;
  width: 100%;
}
input:focus, select:focus { border-color: #3b6fd4; }
input::placeholder { color: #444; }

button {
  background: #1e1e1e;
  border: 1px solid #333;
  border-radius: 6px;
  color: #bbb;
  padding: .4em .9em;
  font-size: .82rem;
  font-family: inherit;
  cursor: pointer;
  transition: background .12s, border-color .12s, color .12s;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: .35rem;
}
button:hover:not(:disabled) { background: #272727; border-color: #484848; color: #ddd; }
button:disabled { opacity: .4; cursor: not-allowed; }

/* Scrollbar */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #2e2e2e; border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: #444; }
::-webkit-scrollbar-corner { background: transparent; }

.btn-primary { background: #1b3d7a; border-color: #2455a8; color: #90b8f0; }
.btn-primary:hover:not(:disabled) { background: #214a90; border-color: #3060b8; color: #b0d0ff; }

.btn-danger { background: transparent; border-color: transparent; color: #666; }
.btn-danger:hover:not(:disabled) { background: #2a1010; border-color: #5a1a1a; color: #f07070; }
</style>

<style scoped>
.app { display: flex; width: 100%; height: 100%; overflow: hidden; }

.sidebar {
  width: 176px;
  min-width: 176px;
  background: #0d0d0d;
  border-right: 1px solid #1e1e1e;
  display: flex;
  flex-direction: column;
  padding: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: .6rem;
  padding: 1rem .875rem .875rem;
  font-weight: 700;
  font-size: .9rem;
  color: #e0e0e0;
  border-bottom: 1px solid #1e1e1e;
  margin-bottom: .5rem;
}
.brand svg { opacity: .7; flex-shrink: 0; }

.nav-group { display: flex; flex-direction: column; gap: 2px; padding: 0 .5rem; }

.nav-item {
  display: flex;
  align-items: center;
  gap: .55rem;
  width: 100%;
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: #686868;
  padding: .5rem .75rem;
  text-align: left;
  font-size: .82rem;
  font-weight: 500;
  cursor: pointer;
  transition: background .1s, color .1s, border-color .1s;
}
.nav-item svg { opacity: .6; flex-shrink: 0; }
.nav-item:hover { background: #161616; color: #bbb; }
.nav-item:hover svg { opacity: .8; }
.nav-item.active { background: #161e2e; border-color: #1e3050; color: #90b8f0; }
.nav-item.active svg { opacity: 1; stroke: #90b8f0; }

.sidebar-footer { margin-top: auto; padding: .75rem .875rem; font-size: .7rem; color: #333; }

.content { flex: 1; overflow-y: auto; background: #111; }
</style>
