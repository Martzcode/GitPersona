mod github;

use anyhow::{anyhow, Context, Result};
use github::{apply_following_filter, validate_date, GitHubClient, SearchCriteria, User};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
  #[serde(default)]
  pub github_username: String,
  #[serde(default)]
  pub token: String,
}

/// Chemin de config.json à côté de l'exécutable.
fn config_path() -> Result<std::path::PathBuf> {
  let exe = std::env::current_exe().context("impossible de localiser l'exécutable")?;
  exe
    .parent()
    .ok_or_else(|| anyhow!("répertoire de l'exécutable introuvable"))
    .map(|dir| dir.join("config.json"))
}

/// Charge config.json ; le crée avec des valeurs vides s'il n'existe pas.
fn load_config() -> Result<AppConfig> {
  let path = config_path()?;
  if !path.exists() {
    log::info!("config.json absent, création d'un fichier par défaut : {}", path.display());
    save_config_file(&AppConfig::default())?;
    return Ok(AppConfig::default());
  }
  let content = std::fs::read_to_string(&path)
    .with_context(|| format!("lecture impossible de {}", path.display()))?;
  if content.trim().is_empty() {
    return Ok(AppConfig::default());
  }
  serde_json::from_str(&content).with_context(|| format!("config.json invalide ({})", path.display()))
}

fn save_config_file(config: &AppConfig) -> Result<()> {
  let path = config_path()?;
  let json = serde_json::to_string_pretty(config).context("sérialisation config.json")?;
  std::fs::write(&path, json + "\n")
    .with_context(|| format!("écriture impossible de {}", path.display()))
}

struct AppState {
  github: GitHubClient,
}

#[tauri::command]
async fn search_users(
  state: State<'_, AppState>,
  criteria: SearchCriteria,
) -> Result<github::SearchUsersResult, String> {
  if let Some(date) = &criteria.last_activity_after {
    validate_date(date).map_err(|e| e.to_string())?;
  }

  let mut result = state
    .github
    .search_users(&criteria)
    .await
    .map_err(|e| e.to_string())?;

  result.users = apply_following_filter(
    result.users,
    criteria.min_following,
    criteria.max_following,
  );

  Ok(result)
}

#[tauri::command]
async fn get_user(state: State<'_, AppState>, login: String) -> Result<User, String> {
  state.github.get_user(&login).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_me(state: State<'_, AppState>) -> Result<github::SimpleUser, String> {
  state
    .github
    .get_authenticated_user()
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_followers(state: State<'_, AppState>) -> Result<Vec<github::SimpleUser>, String> {
  state
    .github
    .list_followers()
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_following(state: State<'_, AppState>) -> Result<Vec<github::SimpleUser>, String> {
  state
    .github
    .list_following()
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_not_followed_back(
  state: State<'_, AppState>,
) -> Result<Vec<github::SimpleUser>, String> {
  state
    .github
    .get_not_followed_back()
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn follow_user(state: State<'_, AppState>, login: String) -> Result<(), String> {
  state
    .github
    .follow_user(&login)
    .await
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn unfollow_user(state: State<'_, AppState>, login: String) -> Result<(), String> {
  state
    .github
    .unfollow_user(&login)
    .await
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn is_authenticated(state: State<'_, AppState>) -> bool {
  state.github.is_authenticated()
}

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
  // Ne pas recréer ici : load_config crée le fichier s'il manque.
  load_config().map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
  save_config_file(&config).map_err(|e| format!("{e:#}"))?;
  state
    .github
    .set_token(if config.token.trim().is_empty() {
      None
    } else {
      Some(config.token)
    });
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let config = load_config().unwrap_or_else(|e| {
    log::warn!("chargement config.json impossible ({e:#}), mode anonyme");
    AppConfig::default()
  });

  let token = if config.token.trim().is_empty() {
    let env_token = std::env::var("GITHUB_TOKEN").ok();
    if env_token.is_none() {
      log::warn!(
        "aucun token (config.json vide et GITHUB_TOKEN non défini), mode anonyme (rate limits bas)"
      );
    }
    env_token
  } else {
    Some(config.token)
  };

  tauri::Builder::default()
    .manage(AppState {
      github: GitHubClient::new(token),
    })
    .invoke_handler(tauri::generate_handler![
      search_users,
      get_user,
      get_me,
      get_followers,
      get_following,
      get_not_followed_back,
      follow_user,
      unfollow_user,
      is_authenticated,
      get_config,
      save_config
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}