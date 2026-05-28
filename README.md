# DSI Toolkit

Ferramenta interna da equipa de Sistemas de Informação da **Câmara Municipal de Abrantes** para gestão de repositórios git e instalação de dependências Maven locais.

---

## O que faz

### 📁 Repositórios
- Descobre automaticamente repositórios git em pastas comuns (`~/Documents`, `~/dev`, `C:\dev`, etc.)
- Mostra o estado de cada repositório em tempo real (actualizado, desactualizado, à frente, divergido)
- Faz **git pull** por repositório individualmente ou em todos ao mesmo tempo (**Pull All**)
- Verifica actualizações automaticamente a cada minuto em segundo plano
- Faz pull automático no arranque nos repositórios configurados (ex: `environmentcma`)

### 📦 JARs Maven
- Lista todos os ficheiros `.jar` de cada projecto com estado visual:
  - 🟢 **Instalado** — jar instalado e actualizado
  - 🟡 **Desactualizado** — o ficheiro foi modificado após a última instalação
  - ⚫ **Em falta** — ainda não foi instalado
- Executa o script `installJars.sh` de cada projecto com barra de progresso em tempo real
- **Instalação inteligente**: se menos de 50% dos JARs precisam de actualização, usa o modo rápido (`-t`) automaticamente ⚡
- Detecta automaticamente a localização do script e da pasta de JARs em cada projecto

### ⚙️ Configuração
- Credenciais Git (HTTPS / PAT GitLab) guardadas localmente
- Lista de repositórios conhecidos com variantes de nome de pasta
- Repositórios com pull automático no arranque
- Versão do Maven
- Botão para testar a ligação ao GitLab

---

## Instalação

Vai à página de [Releases](https://github.com/Dani176343/dsi-toolkit/releases/latest) e transfere o instalador para o teu sistema:

| Sistema | Ficheiro |
|---|---|
| Windows 10/11 | `DSI Toolkit_*_x64-setup.exe` |
| Linux (Ubuntu/Debian) | `dsi-toolkit_*_amd64.deb` |
| Linux (outros) | `dsi-toolkit_*_amd64.AppImage` |
| macOS (M1/M2/M3 e Intel via Rosetta) | `DSI Toolkit_*_aarch64.dmg` |

### Requisitos
- **Windows**: WebView2 Runtime (já incluído no Windows 11; no Windows 10 instala automaticamente)
- **Linux**: sem requisitos adicionais
- **macOS**: sem requisitos adicionais

---

## Actualizações automáticas

A aplicação verifica automaticamente se existe uma versão mais recente **no arranque** e **a cada hora**. Quando há uma nova versão, aparece um painel no canto inferior direito:

```
↓ Nova versão disponível: v0.2.0   [Actualizar agora]  ✕
```

Clica em **Actualizar agora** — a aplicação transfere, instala e pede para reiniciar. Não é necessário fazer nada manualmente.

---

## Desenvolvimento

### Pré-requisitos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/)
- [Git for Windows](https://git-scm.com/) (no Windows, necessário para executar os scripts bash)

### Arrancar em modo de desenvolvimento

```bash
npm install
npm run tauri dev
```

### Build local

```bash
npm run tauri build
# Instaladores em: src-tauri/target/release/bundle/
```

### Lançar uma nova versão

```bash
# 1. Actualiza a versão em:
#    - src-tauri/tauri.conf.json  →  "version": "X.Y.Z"
#    - src-tauri/Cargo.toml       →  version = "X.Y.Z"

# 2. Commit + tag
git add .
git commit -m "chore: bump version to vX.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

O GitHub Actions compila automaticamente para Windows, Linux e macOS e publica a Release. Os utilizadores recebem o aviso de actualização na aplicação.

---

## Tecnologias

| Camada | Tecnologia |
|---|---|
| Framework desktop | [Tauri v2](https://tauri.app/) |
| Frontend | [Vue 3](https://vuejs.org/) + TypeScript |
| Backend | Rust |
| Git | [git2](https://github.com/rust-lang/git2-rs) (libgit2) |
| CI/CD | GitHub Actions |

---

## Estrutura do projecto

```
dsi-toolkit/
├── src/                        # Frontend Vue 3
│   ├── views/
│   │   ├── ReposView.vue       # Ecrã de repositórios
│   │   ├── JarsView.vue        # Ecrã de JARs Maven
│   │   └── ConfigView.vue      # Ecrã de configuração
│   └── components/
│       └── UpdateBanner.vue    # Painel de actualização automática
├── src-tauri/                  # Backend Rust
│   └── src/commands/
│       ├── config.rs           # Configuração (leitura/escrita, detecção de scripts)
│       ├── git.rs              # Operações git (estado, pull, credenciais)
│       ├── jars.rs             # Listagem e instalação de JARs
│       └── autodiscovery.rs    # Descoberta automática de repositórios
└── .github/workflows/
    └── build.yml               # Pipeline de build multi-plataforma
```

---

*Câmara Municipal de Abrantes — Divisão de Sistemas de Informação*
