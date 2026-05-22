use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub path: String,
    /// Path do installJars.sh deste repo (detetado automaticamente ou manual).
    #[serde(default)]
    pub install_script: Option<String>,
    /// Diretório com os .jar deste repo (detetado automaticamente ou manual).
    #[serde(default)]
    pub jars_dir: Option<String>,
}

/// Credenciais HTTPS para autenticação git.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCredentials {
    pub host: String,
    pub username: String,
    pub token: String,
}

/// Define um repo a descobrir: nome canónico + variantes de pasta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownRepo {
    pub name: String,
    pub folder_names: Vec<String>,
}

impl KnownRepo {
    fn new(name: &str, folder_names: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            folder_names: folder_names.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub repos: Vec<RepoConfig>,
    pub known_repos: Vec<KnownRepo>,
    pub auto_pull_repos: Vec<String>,
    pub git_credentials: Option<GitCredentials>,
    pub maven_version: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            repos: vec![],
            known_repos: vec![
                KnownRepo::new("projetos-sdk",     &["projetos-sdk", "projetos-sdk-master"]),
                KnownRepo::new("projetos-sdk-dev", &["projetos-sdk-dev", "projetos-sdk-dev-master"]),
                KnownRepo::new("environmentcma",   &["environmentcma"]),
            ],
            auto_pull_repos: vec!["environmentcma".to_string()],
            git_credentials: None,
            maven_version: "3.9.6".to_string(),
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json"))
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return AppConfig::default(),
    };
    if !path.exists() {
        return AppConfig::default();
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut config: AppConfig = serde_json::from_str(&content).unwrap_or_default();

    // Garante que known_repos do código estão sempre presentes,
    // mesmo em configs antigos que não os tinham ainda.
    let defaults = AppConfig::default();
    for default_repo in &defaults.known_repos {
        if !config.known_repos.iter().any(|k| k.name == default_repo.name) {
            config.known_repos.push(default_repo.clone());
        }
    }

    config
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// Subdiretórios comuns onde costumam estar JARs, por ordem de preferência.
const JAR_SUBDIRS: &[&str] = &["jars", "libs", "lib", "dependencies", "maven", "."];

fn has_jars(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jar"))
        })
        .unwrap_or(false)
}

fn detect_jars_dir(repo_path: &str) -> Option<String> {
    let base = Path::new(repo_path);
    for subdir in JAR_SUBDIRS {
        let candidate = if *subdir == "." {
            base.to_path_buf()
        } else {
            base.join(subdir)
        };
        if candidate.is_dir() && has_jars(&candidate) {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> AppConfig {
    load_config(&app)
}

#[tauri::command]
pub fn set_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    save_config(&app, &config)
}

#[tauri::command]
pub fn add_repo(app: AppHandle, name: String, path: String) -> Result<AppConfig, String> {
    let mut config = load_config(&app);
    config.repos.retain(|r| r.name != name);
    config.repos.push(RepoConfig {
        name,
        path,
        install_script: None,
        jars_dir: None,
    });
    save_config(&app, &config)?;
    Ok(config)
}

/// Deteta automaticamente installJars.sh e diretório de JARs em cada repo.
#[tauri::command]
pub fn detect_install_scripts(app: AppHandle) -> Result<AppConfig, String> {
    let mut config = load_config(&app);
    for repo in &mut config.repos {
        // Script
        if repo.install_script.is_none() {
            let candidate = Path::new(&repo.path).join("installJars.sh");
            if candidate.exists() {
                repo.install_script = Some(candidate.to_string_lossy().to_string());
            }
        }
        // Diretório de JARs
        if repo.jars_dir.is_none() {
            repo.jars_dir = detect_jars_dir(&repo.path);
        }
    }
    save_config(&app, &config)?;
    Ok(config)
}

#[tauri::command]
pub fn remove_repo(app: AppHandle, name: String) -> Result<AppConfig, String> {
    let mut config = load_config(&app);
    config.repos.retain(|r| r.name != name);
    save_config(&app, &config)?;
    Ok(config)
}
