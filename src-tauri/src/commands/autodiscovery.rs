use super::config::{load_config, save_config, KnownRepo, RepoConfig};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Clone)]
pub struct DiscoveryResult {
    pub found: Vec<RepoConfig>,
    pub missing: Vec<String>,
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("dev"));
        paths.push(home.join("projects"));
        paths.push(home.join("Documents"));
        paths.push(home.join("source"));
        paths.push(home.join("workspace"));
    }

    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from(r"C:\dev"));
        paths.push(PathBuf::from(r"C:\projects"));
        paths.push(PathBuf::from(r"D:\dev"));
        paths.push(PathBuf::from(r"D:\projects"));
    }

    paths
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Devolve o nome canónico do `KnownRepo` cujo `folder_names` contém `folder`, ou `None`.
fn canonical_name<'a>(folder: &str, known: &'a [KnownRepo]) -> Option<&'a str> {
    known
        .iter()
        .find(|k| k.folder_names.iter().any(|f| f == folder))
        .map(|k| k.name.as_str())
}

/// Procura repos conhecidos em `base` e num nível de subpastas.
/// Devolve pares (nome_canónico, path).
fn search_in(base: &Path, known: &[KnownRepo]) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();

    if !base.is_dir() {
        return found;
    }

    let Ok(entries) = std::fs::read_dir(base) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if let Some(canon) = canonical_name(&folder, known) {
            if is_git_repo(&path) {
                found.push((canon.to_string(), path.clone()));
            }
        }

        // Um nível de subpastas
        if let Ok(subs) = std::fs::read_dir(&path) {
            for sub in subs.flatten() {
                let sp = sub.path();
                if !sp.is_dir() {
                    continue;
                }
                let sf = sp
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(canon) = canonical_name(&sf, known) {
                    if is_git_repo(&sp) {
                        found.push((canon.to_string(), sp));
                    }
                }
            }
        }
    }

    found
}

/// Procura automaticamente repos conhecidos em paths comuns.
/// - Guarda os encontrados em config.json.
/// - Emite evento `repos-not-found` com a lista dos não encontrados.
#[tauri::command]
pub async fn autodiscover_repos(app: AppHandle) -> Result<DiscoveryResult, String> {
    let mut config = load_config(&app);

    if config.known_repos.is_empty() {
        return Ok(DiscoveryResult {
            found: vec![],
            missing: vec![],
        });
    }

    // Repos já configurados (pelo nome canónico) — não re-procurar
    let already_configured: Vec<String> = config.repos.iter().map(|r| r.name.clone()).collect();
    let to_find: Vec<KnownRepo> = config
        .known_repos
        .iter()
        .filter(|k| !already_configured.contains(&k.name))
        .cloned()
        .collect();

    if to_find.is_empty() {
        return Ok(DiscoveryResult {
            found: vec![],
            missing: vec![],
        });
    }

    let mut found_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for root in candidate_roots() {
        for (canon, path) in search_in(&root, &to_find) {
            found_map
                .entry(canon)
                .or_insert_with(|| path.to_string_lossy().to_string());
        }
    }

    let mut found = Vec::new();
    let mut missing = Vec::new();

    for known in &to_find {
        if let Some(path) = found_map.get(&known.name) {
            let repo = RepoConfig {
                name: known.name.clone(),
                path: path.clone(),
                install_script: None,
                jars_dir: None,
            };
            config.repos.push(repo.clone());
            found.push(repo);
        } else {
            missing.push(known.name.clone());
        }
    }

    save_config(&app, &config)?;

    if !missing.is_empty() {
        app.emit("repos-not-found", missing.clone())
            .map_err(|e| e.to_string())?;
    }

    Ok(DiscoveryResult { found, missing })
}
