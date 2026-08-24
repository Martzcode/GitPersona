mod github;

use github::{apply_following_filter, validate_date, GitHubClient, SearchCriteria, User};
use tauri::State;

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
fn is_authenticated(state: State<'_, AppState>) -> bool {
  state.github.is_authenticated()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let token = std::env::var("GITHUB_TOKEN").ok();
  if token.is_none() {
    log::warn!("GITHUB_TOKEN not set, running in anonymous mode (rate limits: 10 req/min search)");
  }

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
      is_authenticated
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