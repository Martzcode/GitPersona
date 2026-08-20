use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::Mutex;

const API_BASE: &str = "https://api.github.com";
const SEARCH_MIN_INTERVAL: Duration = Duration::from_millis(2100);

pub struct GitHubClient {
  http: reqwest::Client,
  token: Option<String>,
  search_limiter: Mutex<()>,
  last_search: Mutex<Option<std::time::Instant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCriteria {
  pub location: Option<String>,
  pub min_repos: Option<u32>,
  pub max_repos: Option<u32>,
  pub min_followers: Option<u32>,
  pub max_followers: Option<u32>,
  pub min_following: Option<u32>,
  pub max_following: Option<u32>,
  pub last_activity_after: Option<String>,
  pub per_page: Option<u32>,
  pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchUsersResult {
  pub total_count: u64,
  pub users: Vec<User>,
  pub remaining: Option<u32>,
  pub partial: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
  pub login: String,
  pub id: u64,
  pub avatar_url: String,
  pub html_url: String,
  pub name: Option<String>,
  pub bio: Option<String>,
  pub location: Option<String>,
  pub public_repos: u32,
  pub followers: u32,
  pub following: u32,
  pub pushed_at: Option<String>,
  pub created_at: Option<String>,
  pub company: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
  total_count: u64,
  items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
  login: String,
  avatar_url: String,
  html_url: String,
}

#[derive(Debug, Deserialize)]
struct RawUser {
  login: String,
  id: u64,
  avatar_url: String,
  html_url: String,
  name: Option<String>,
  bio: Option<String>,
  location: Option<String>,
  public_repos: u32,
  followers: u32,
  following: u32,
  pushed_at: Option<String>,
  created_at: Option<String>,
  company: Option<String>,
}

impl GitHubClient {
  pub fn new(token: Option<String>) -> Self {
    let http = reqwest::Client::builder()
      .user_agent("GitPersona/0.1")
      .connect_timeout(Duration::from_secs(10))
      .build()
      .expect("failed to build http client");

    Self {
      http,
      token,
      search_limiter: Mutex::new(()),
      last_search: Mutex::new(None),
    }
  }

  pub fn is_authenticated(&self) -> bool {
    self.token.is_some()
  }

  async fn wait_search_slot(&self) -> Result<()> {
    let _guard = self.search_limiter.lock().await;
    let mut last = self.last_search.lock().await;
    if let Some(prev) = *last {
      let elapsed = prev.elapsed();
      if elapsed < SEARCH_MIN_INTERVAL {
        tokio::time::sleep(SEARCH_MIN_INTERVAL - elapsed).await;
      }
    }
    *last = Some(std::time::Instant::now());
    Ok(())
  }

  pub async fn search_users(&self, criteria: &SearchCriteria) -> Result<SearchUsersResult> {
    self.wait_search_slot().await?;

    let query = build_query(criteria);
    let mut per_page = criteria.per_page.unwrap_or(30).clamp(1, 100);
    if self.token.is_none() {
      per_page = per_page.min(10);
    }
    let page = criteria.page.unwrap_or(1);

    let mut req = self
      .http
      .get(format!("{API_BASE}/search/users"))
      .query(&[
        ("q", query.as_str()),
        ("per_page", per_page.to_string().as_str()),
        ("page", page.to_string().as_str()),
      ]);
    if let Some(token) = &self.token {
      req = req.bearer_auth(token);
    }

    let resp = req
      .send()
      .await
      .context("search request failed")?
      .error_for_status()
      .map_err(|e| {
        if let Some(status) = e.status() {
          if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
          {
            anyhow!("API GitHub saturé (rate limit). {} : attendez ~1h ou définissez GITHUB_TOKEN.", if self.token.is_none() { "Mode anonyme actif (60 req/h)" } else { "Token actif" })
          } else {
            anyhow!("erreur GitHub ({}): {e}", status.as_u16())
          }
        } else {
          anyhow!("erreur réseau : {e}")
        }
      })?;

    let remaining = resp
      .headers()
      .get("x-ratelimit-remaining")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.parse::<u32>().ok());

    let search: SearchResponse = resp.json().await.context("failed to parse search response")?;

    let users = match self.fetch_users(&search.items).await {
      Ok(users) => users,
      Err(e) => {
        log::warn!("profile fetch failed, returning partial results: {e}");
        search
          .items
          .into_iter()
          .map(|item| User {
            login: item.login,
            id: 0,
            avatar_url: item.avatar_url,
            html_url: item.html_url,
            name: None,
            bio: None,
            location: None,
            public_repos: 0,
            followers: 0,
            following: 0,
            pushed_at: None,
            created_at: None,
            company: None,
          })
          .collect()
      }
    };

    let partial = users.iter().any(|u| u.id == 0);

    Ok(SearchUsersResult {
      total_count: search.total_count,
      users,
      remaining,
      partial,
    })
  }

  pub async fn get_user(&self, login: &str) -> Result<User> {
    let mut req = self.http.get(format!("{API_BASE}/users/{login}"));
    if let Some(token) = &self.token {
      req = req.bearer_auth(token);
    }
    let resp = req
      .send()
      .await
      .context("user request failed")?
      .error_for_status()
      .map_err(|e| {
        if let Some(status) = e.status() {
          if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
          {
            anyhow!("API GitHub saturé (rate limit). {} : attendez ~1h ou définissez GITHUB_TOKEN.", if self.token.is_none() { "Mode anonyme actif (60 req/h)" } else { "Token actif" })
          } else {
            anyhow!("erreur GitHub ({}): {e}", status.as_u16())
          }
        } else {
          anyhow!("erreur réseau : {e}")
        }
      })?;
    let raw: RawUser = resp.json().await.context("failed to parse user response")?;
    Ok(raw.into())
  }

  async fn fetch_users(&self, items: &[SearchItem]) -> Result<Vec<User>> {
    let http = self.http.clone();
    let token = self.token.clone();
    let mut tasks = tokio::task::JoinSet::new();
    for item in items {
      let login = item.login.clone();
      let http = http.clone();
      let token = token.clone();
      tasks.spawn(async move {
        let mut req = http.get(format!("{API_BASE}/users/{login}"));
        if let Some(token) = &token {
          req = req.bearer_auth(token);
        }
        let resp = req
          .send()
          .await
          .context("user request failed")?
          .error_for_status()
          .context("user request error")?;
        let raw: RawUser = resp.json().await.context("failed to parse user")?;
        Ok::<User, anyhow::Error>(raw.into())
      });
    }

    let mut users = Vec::with_capacity(items.len());
    let mut failures = 0usize;
    while let Some(res) = tasks.join_next().await {
      match res {
        Ok(Ok(user)) => users.push(user),
        Ok(Err(e)) => {
          failures += 1;
          log::warn!("failed to fetch user: {e}");
        }
        Err(e) => {
          failures += 1;
          log::warn!("task error: {e}");
        }
      }
    }

    if failures > 0 && failures * 2 >= items.len() {
      return Err(anyhow!(
        "la majorité des profils n'a pas pu être chargée ({} sur {}). Le rate limit de l'API GitHub est probablement atteint. {}",
        failures,
        items.len(),
        if self.token.is_none() {
          "Mode anonyme : 60 req/h seulement. Définissez GITHUB_TOKEN."
        } else {
          "Réessayez dans quelques minutes."
        }
      ));
    }

    Ok(users)
  }
}

impl From<RawUser> for User {
  fn from(raw: RawUser) -> Self {
    Self {
      login: raw.login,
      id: raw.id,
      avatar_url: raw.avatar_url,
      html_url: raw.html_url,
      name: raw.name,
      bio: raw.bio,
      location: raw.location,
      public_repos: raw.public_repos,
      followers: raw.followers,
      following: raw.following,
      pushed_at: raw.pushed_at,
      created_at: raw.created_at,
      company: raw.company,
    }
  }
}

fn build_query(criteria: &SearchCriteria) -> String {
  let mut parts: Vec<String> = Vec::new();

  if let Some(loc) = &criteria.location {
    if !loc.trim().is_empty() {
      parts.push(format!("location:{}", loc.trim()));
    }
  }
  if let Some(min) = criteria.min_repos {
    parts.push(format!("repos:>={min}"));
  }
  if let Some(max) = criteria.max_repos {
    parts.push(format!("repos:<={max}"));
  }
  if let Some(min) = criteria.min_followers {
    parts.push(format!("followers:>={min}"));
  }
  if let Some(max) = criteria.max_followers {
    parts.push(format!("followers:<={max}"));
  }
  if let Some(date) = &criteria.last_activity_after {
    if !date.trim().is_empty() {
      parts.push(format!("pushed:>{}", date.trim()));
    }
  }

  if parts.is_empty() {
    parts.push("type:user".to_string());
  } else {
    parts.push("type:user".to_string());
  }

  parts.join(" ")
}

pub fn apply_following_filter(
  users: Vec<User>,
  min_following: Option<u32>,
  max_following: Option<u32>,
) -> Vec<User> {
  users
    .into_iter()
    .filter(|u| min_following.map_or(true, |min| u.following >= min))
    .filter(|u| max_following.map_or(true, |max| u.following <= max))
    .collect()
}

pub fn validate_date(date: &str) -> Result<()> {
  if date.is_empty() {
    return Ok(());
  }
  let valid = date
    .parse::<chrono::NaiveDate>()
    .is_ok()
    || date
      .parse::<chrono::DateTime<chrono::Utc>>()
      .is_ok();
  if !valid {
    return Err(anyhow!("invalid date format: {date} (expected YYYY-MM-DD)"));
  }
  Ok(())
}
