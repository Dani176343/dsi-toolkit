use git2::{AutotagOption, FetchOptions, RemoteCallbacks, Repository};
use serde::Serialize;
use tauri::{AppHandle};

use super::config::{load_config, GitCredentials};

#[derive(Debug, Serialize)]
pub struct RepoStatus {
    pub name: String,
    pub path: String,
    pub status: StatusKind,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StatusKind {
    UpToDate,
    Behind { commits: usize },
    Ahead { commits: usize },
    Diverged { behind: usize, ahead: usize },
    NoRemote,
    Error { message: String },
}

/// Constrói FetchOptions com a cadeia de autenticação correcta:
/// 1. Credenciais guardadas na app (se o host corresponder)
/// 2. SSH agent
/// 3. Credential helper do sistema (Windows Credential Manager, etc.)
fn fetch_options(creds: Option<GitCredentials>) -> FetchOptions<'static> {
    let mut cb = RemoteCallbacks::new();

    cb.credentials(move |url, username, allowed| {
        // 1. Credenciais configuradas na UI
        if let Some(ref c) = creds {
            if url.contains(c.host.as_str())
                && allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
            {
                return git2::Cred::userpass_plaintext(&c.username, &c.token);
            }
        }

        // 2. SSH agent
        if allowed.contains(git2::CredentialType::SSH_KEY) {
            if let Some(user) = username {
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(user) {
                    return Ok(cred);
                }
            }
        }

        // 3. Credential helper do sistema
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let Ok(cfg) = git2::Config::open_default() {
                if let Ok(cred) = git2::Cred::credential_helper(&cfg, url, username) {
                    return Ok(cred);
                }
            }
        }

        Err(git2::Error::from_str(
            "Sem credenciais. Configura o login no separador Config.",
        ))
    });

    let mut opts = FetchOptions::new();
    opts.remote_callbacks(cb);
    opts.download_tags(AutotagOption::None);
    opts.prune(git2::FetchPrune::Off);
    opts
}

/// Faz fetch e devolve o estado local vs. remoto do repo.
#[tauri::command]
pub async fn get_repo_status(app: AppHandle, name: String, path: String) -> RepoStatus {
    let creds = load_config(&app).git_credentials;
    let result = (|| -> Result<StatusKind, String> {
        let repo = Repository::open(&path).map_err(|e| e.message().to_string())?;

        if let Ok(mut remote) = repo.find_remote("origin") {
            let mut opts = fetch_options(creds.clone());
            let _ = remote.fetch(&[] as &[&str], Some(&mut opts), None);
        }

        let head = repo.head().map_err(|e| e.message().to_string())?;
        let local_oid = head.target().ok_or("HEAD sem target")?;
        let branch_name = head.shorthand().unwrap_or("main");
        let remote_ref = format!("refs/remotes/origin/{}", branch_name);

        let remote_ref_obj = match repo.find_reference(&remote_ref) {
            Ok(r) => r,
            Err(_) => return Ok(StatusKind::NoRemote),
        };

        let remote_oid = remote_ref_obj.target().ok_or("Remote ref sem target")?;

        if local_oid == remote_oid {
            return Ok(StatusKind::UpToDate);
        }

        let (ahead, behind) = repo
            .graph_ahead_behind(local_oid, remote_oid)
            .map_err(|e| e.message().to_string())?;

        Ok(match (ahead, behind) {
            (0, b) => StatusKind::Behind { commits: b },
            (a, 0) => StatusKind::Ahead { commits: a },
            (a, b) => StatusKind::Diverged { behind: b, ahead: a },
        })
    })();

    RepoStatus {
        name,
        path,
        status: result.unwrap_or_else(|message| StatusKind::Error { message }),
    }
}

/// Fast-forward pull do origin.
#[tauri::command]
pub async fn git_pull(app: AppHandle, name: String, path: String) -> Result<String, String> {
    let creds = load_config(&app).git_credentials;
    let repo = Repository::open(&path).map_err(|e| e.message().to_string())?;

    let mut remote = repo
        .find_remote("origin")
        .map_err(|e| e.message().to_string())?;

    let mut opts = fetch_options(creds);
    remote
        .fetch(&[] as &[&str], Some(&mut opts), None)
        .map_err(|e| e.message().to_string())?;

    let head = repo.head().map_err(|e| e.message().to_string())?;
    let branch_name = head.shorthand().unwrap_or("main").to_string();
    let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);

    let fetch_head = repo
        .find_reference(&remote_ref_name)
        .map_err(|e| e.message().to_string())?;
    let fetch_commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .map_err(|e| e.message().to_string())?;

    let (analysis, _) = repo
        .merge_analysis(&[&fetch_commit])
        .map_err(|e| e.message().to_string())?;

    if analysis.is_up_to_date() {
        return Ok(format!("{}: já está atualizado", name));
    }

    if analysis.is_fast_forward() {
        let refname = head
            .name()
            .ok_or("HEAD sem nome de referência")?
            .to_string();

        let mut reference = repo
            .find_reference(&refname)
            .map_err(|e| e.message().to_string())?;

        reference
            .set_target(fetch_commit.id(), "pull: fast-forward")
            .map_err(|e| e.message().to_string())?;
        repo.set_head(&refname)
            .map_err(|e| e.message().to_string())?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| e.message().to_string())?;

        return Ok(format!("{}: fast-forward concluído", name));
    }

    Err(format!(
        "{}: merge necessário — existem commits locais.",
        name
    ))
}

/// Testa as credenciais configuradas fazendo um ls-remote ao primeiro repo disponível.
#[tauri::command]
pub async fn test_git_connection(app: AppHandle) -> Result<String, String> {
    let config = load_config(&app);

    let creds = config
        .git_credentials
        .clone()
        .ok_or("Nenhuma credencial configurada.")?;

    let repo = config
        .repos
        .first()
        .ok_or("Nenhum repo configurado para testar.")?;

    let git_repo = Repository::open(&repo.path).map_err(|e| e.message().to_string())?;
    let mut remote = git_repo
        .find_remote("origin")
        .map_err(|e| e.message().to_string())?;

    let mut opts = fetch_options(Some(creds));
    remote
        .fetch(&[] as &[&str], Some(&mut opts), None)
        .map_err(|e| format!("Falhou: {}", e.message()))?;

    Ok(format!("Ligação OK — autenticado em {}", repo.name))
}
