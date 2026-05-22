// Espelha as structs Rust — manter em sync com src-tauri/src/commands/

export interface RepoConfig {
  name: string;
  path: string;
  install_script: string | null;
  jars_dir: string | null;
}

export interface GitCredentials {
  host: string;
  username: string;
  token: string;
}

export interface KnownRepo {
  name: string;
  folder_names: string[];
}

export interface AppConfig {
  repos: RepoConfig[];
  known_repos: KnownRepo[];
  auto_pull_repos: string[];
  git_credentials: GitCredentials | null;
  maven_version: string;
}

// git.rs ---------------------------------------------------------------

export type StatusKind =
  | { type: "upToDate" }
  | { type: "behind"; commits: number }
  | { type: "ahead"; commits: number }
  | { type: "diverged"; behind: number; ahead: number }
  | { type: "noRemote" }
  | { type: "error"; message: string };

export interface RepoStatus {
  name: string;
  path: string;
  status: StatusKind;
}

// jars.rs --------------------------------------------------------------

export interface JarInfo {
  name: string;
  path: string;
  size_bytes: number;
  last_modified: number | null;   // Unix seconds
  install_timestamp: number | null;
}

// autodiscovery.rs -----------------------------------------------------

export interface DiscoveryResult {
  found: RepoConfig[];
  missing: string[];
}
