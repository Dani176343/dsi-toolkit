use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

// ─── Tipos ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct JarInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub last_modified: Option<u64>,
    pub install_timestamp: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct InstallProgressEvent {
    pub repo: String,
    pub installed: usize,
    pub total: usize,
    pub jar: String,
}

#[derive(Serialize, Clone)]
pub struct InstallDoneEvent {
    pub repo: String,
    pub installed: usize,
    pub total: usize,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn timestamps_dir(jars_dir: &Path) -> PathBuf {
    jars_dir.join(".install_jars_timestamps")
}

fn file_mtime_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Extrai o nome do JAR de uma linha de output do installJars.sh.
/// Suporta várias formas:
///   "✔ Instalado ./NOME.jar como ..."
///   "Installed ./NOME.jar"
///   "[INFO] Installing ... NOME.jar"
fn parse_jar_name(line: &str) -> Option<String> {
    let line = line.trim();

    // Formato principal: "Instalado ./<nome>.jar"
    if let Some(idx) = line.find("Instalado ./") {
        let rest = &line[idx + "Instalado ./".len()..];
        if let Some(end) = rest.find(".jar") {
            return Some(format!("{}.jar", &rest[..end]));
        }
    }

    // Formato alternativo inglês: "Installed ./<nome>.jar"
    if let Some(idx) = line.find("Installed ./") {
        let rest = &line[idx + "Installed ./".len()..];
        if let Some(end) = rest.find(".jar") {
            return Some(format!("{}.jar", &rest[..end]));
        }
    }

    // Formato Maven install:plugin: "[INFO] Installing /path/to/NOME.jar to ..."
    if line.contains("[INFO] Installing") && line.contains(".jar") {
        if let Some(start) = line.rfind('/').or_else(|| line.rfind('\\')) {
            let rest = &line[start + 1..];
            if let Some(end) = rest.find(".jar") {
                return Some(format!("{}.jar", &rest[..end]));
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn resolve_bash(script_path: &str) -> (String, Vec<String>) {
    const CANDIDATES: &[&str] = &[
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
        r"C:\Git\bin\bash.exe",
    ];
    for c in CANDIDATES {
        if Path::new(c).is_file() {
            return (c.to_string(), vec![script_path.to_string()]);
        }
    }
    ("bash".to_string(), vec![script_path.to_string()])
}

#[cfg(not(target_os = "windows"))]
fn resolve_bash(script_path: &str) -> (String, Vec<String>) {
    ("bash".to_string(), vec![script_path.to_string()])
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// Lista todos os .jar em `jars_dir` com estado de instalação.
#[tauri::command]
pub fn list_jars(jars_dir: String) -> Result<Vec<JarInfo>, String> {
    let dir = Path::new(&jars_dir);
    if !dir.is_dir() {
        return Err(format!("Diretório não encontrado: {}", jars_dir));
    }

    let ts_dir = timestamps_dir(dir);
    let mut jars: Vec<JarInfo> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "jar" {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            let meta = std::fs::metadata(&path).ok()?;
            Some(JarInfo {
                install_timestamp: file_mtime_secs(&ts_dir.join(&name)),
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes: meta.len(),
                last_modified: file_mtime_secs(&path),
            })
        })
        .collect();

    jars.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(jars)
}

/// Executa installJars.sh em streaming.
/// Se `incremental` for true, passa o flag `-t` ao script (só instala os mais recentes).
/// - emite evento `install-progress` por cada JAR instalado
/// - cria ficheiro de timestamp em .install_jars_timestamps/
/// - emite `install-done` no final
/// - retorna Err com mensagem se o script falhar
#[tauri::command]
pub async fn run_install_jars(
    app: AppHandle,
    repo_name: String,
    jars_dir: String,
    script_path: String,
    incremental: bool,
) -> Result<usize, String> {
    if !Path::new(&script_path).is_file() {
        return Err(format!("Script não encontrado: {}", script_path));
    }

    // Conta JARs para saber o total antecipadamente
    let total = std::fs::read_dir(&jars_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path().extension().and_then(|x| x.to_str()) == Some("jar")
                })
                .count()
        })
        .unwrap_or(0);

    let ts_dir = timestamps_dir(Path::new(&jars_dir));
    let _ = std::fs::create_dir_all(&ts_dir);

    let (program, mut args) = resolve_bash(&script_path);

    // Passa -t ao script quando modo incremental
    if incremental {
        args.push("-t".to_string());
    }

    let mut child = tokio::process::Command::new(&program)
        .args(&args)
        .current_dir(&jars_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Erro ao lançar bash: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();

    let mut installed = 0;

    // Lê stderr numa task separada
    let stderr = child.stderr.take().unwrap();
    let stderr_handle = tokio::spawn(async move {
        let mut buf = Vec::new();
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            buf.push(line);
        }
        buf
    });

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(jar_name) = parse_jar_name(&line) {
            installed += 1;

            // Cria/atualiza ficheiro de timestamp
            let _ = std::fs::write(ts_dir.join(&jar_name), "");

            app.emit(
                "install-progress",
                InstallProgressEvent {
                    repo: repo_name.clone(),
                    installed,
                    total,
                    jar: jar_name,
                },
            )
            .ok();
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| e.to_string())?;

    let stderr_lines = stderr_handle.await.unwrap_or_default();

    if status.success() {
        app.emit(
            "install-done",
            InstallDoneEvent {
                repo: repo_name,
                installed,
                total,
            },
        )
        .ok();
        Ok(installed)
    } else {
        let msg = if stderr_lines.is_empty() {
            format!("Script terminou com código {}", status.code().unwrap_or(-1))
        } else {
            let filtered: Vec<_> = stderr_lines
                .iter()
                .filter(|l| l.contains("ERROR") || l.contains("FAILURE") || l.contains("Exception"))
                .cloned()
                .collect();
            if filtered.is_empty() {
                stderr_lines.join("\n")
            } else {
                filtered.join("\n")
            }
        };
        Err(msg)
    }
}
