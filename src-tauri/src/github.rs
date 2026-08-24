use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};

const API_BASE: &str = "https://api.github.com";
const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const SEARCH_MIN_INTERVAL: Duration = Duration::from_millis(2100);
const CONCURRENCY_WITH_TOKEN: usize = 8;
const CONCURRENCY_ANONYMOUS: usize = 4;

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
  #[serde(default)]
  pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchUsersResult {
  pub total_count: u64,
  pub users: Vec<User>,
  pub remaining: Option<u32>,
  pub partial: bool,
  pub end_cursor: Option<String>,
  pub has_next: bool,
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct RawSimpleUser {
  login: String,
  id: u64,
  avatar_url: String,
  html_url: String,
}

#[derive(Debug, Serialize)]
pub struct SimpleUser {
  pub login: String,
  pub id: u64,
  pub avatar_url: String,
  pub html_url: String,
}

impl From<RawSimpleUser> for SimpleUser {
  fn from(raw: RawSimpleUser) -> Self {
    Self {
      login: raw.login,
      id: raw.id,
      avatar_url: raw.avatar_url,
      html_url: raw.html_url,
    }
  }
}

#[derive(Debug, Deserialize)]
struct RateLimitResponse {
  resources: RateLimitResources,
}

#[derive(Debug, Deserialize)]
struct RateLimitResources {
  core: RateLimitCore,
}

#[derive(Debug, Deserialize)]
struct RateLimitCore {
  remaining: u32,
}

// --- GraphQL ---

const GRAPHQL_QUERY: &str = r#"
query($q: String!, $first: Int!, $after: String) {
  search(query: $q, type: USER, first: $first, after: $after) {
    userCount
    pageInfo { endCursor hasNextPage }
    edges {
      node {
        ... on User {
          login
          databaseId
          avatarUrl
          url
          name
          bio
          company
          location
          createdAt
          followers { totalCount }
          following { totalCount }
          repositories(privacy: PUBLIC, first: 1, orderBy: {field: PUSHED_AT, direction: DESC}) {
            totalCount
            nodes { pushedAt }
          }
        }
      }
    }
  }
}
"#;

#[derive(Debug, Deserialize)]
struct GqlResponse {
  data: Option<GqlData>,
  errors: Option<Vec<GqlError>>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
  message: String,
}

#[derive(Debug, Deserialize)]
struct GqlData {
  search: Option<GqlSearch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlSearch {
  user_count: u64,
  page_info: GqlPageInfo,
  #[serde(default)]
  edges: Vec<GqlEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlPageInfo {
  end_cursor: Option<String>,
  has_next_page: bool,
}

#[derive(Debug, Deserialize)]
struct GqlEdge {
  node: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
  login: String,
  database_id: Option<u64>,
  avatar_url: String,
  url: String,
  name: Option<String>,
  bio: Option<String>,
  company: Option<String>,
  location: Option<String>,
  created_at: Option<String>,
  followers: Option<GqlCount>,
  following: Option<GqlCount>,
  repositories: Option<GqlRepos>,
}

#[derive(Debug, Deserialize)]
struct GqlCount {
  total_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlRepos {
  total_count: u32,
  #[serde(default)]
  nodes: Vec<GqlRepoNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlRepoNode {
  pushed_at: Option<String>,
}

// --- Erreurs de fetch de profil ---

#[derive(Debug)]
enum ProfileError {
  Skipped,
  RateLimited,
  Other,
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

  fn auth_get(&self, path: &str) -> reqwest::RequestBuilder {
    let mut req = self.http.get(format!("{API_BASE}{path}"));
    if let Some(token) = &self.token {
      req = req.bearer_auth(token);
    }
    req
  }

  /// Profil de l'utilisateur authentifié (requiert un token).
  pub async fn get_authenticated_user(&self) -> Result<SimpleUser> {
    let resp = self
      .auth_get("/user")
      .send()
      .await
      .context("authenticated user request failed")?
      .error_for_status()
      .map_err(|e| match e.status() {
        Some(status) => self.rate_limit_error(status, "erreur profil authentifié"),
        None => anyhow!("erreur réseau : {e}"),
      })?;
    let raw: RawSimpleUser = resp
      .json()
      .await
      .context("failed to parse authenticated user")?;
    Ok(raw.into())
  }

  /// Liste paginée complète (100/page) d'une route de relations.
  async fn list_all(&self, path: &str) -> Result<Vec<SimpleUser>> {
    let mut all = Vec::new();
    let mut page = 1u32;
    loop {
      let page_str = page.to_string();
      let resp = self
        .auth_get(path)
        .query(&[("per_page", "100"), ("page", page_str.as_str())])
        .send()
        .await
        .with_context(|| format!("request failed for {path} (page {page})"))?
        .error_for_status()
        .map_err(|e| match e.status() {
          Some(status) => self.rate_limit_error(status, "erreur liste relations"),
          None => anyhow!("erreur réseau : {e}"),
        })?;
      let users: Vec<RawSimpleUser> = resp
        .json()
        .await
        .context("failed to parse relations response")?;
      let count = users.len();
      all.extend(users.into_iter().map(Into::into));
      if count < 100 || page >= 200 {
        break;
      }
      page += 1;
    }
    Ok(all)
  }

  pub async fn list_followers(&self) -> Result<Vec<SimpleUser>> {
    self.list_all("/user/followers").await
  }

  pub async fn list_following(&self) -> Result<Vec<SimpleUser>> {
    self.list_all("/user/following").await
  }

  /// Personnes suivies qui ne nous suivent pas en retour.
  pub async fn get_not_followed_back(&self) -> Result<Vec<SimpleUser>> {
    let (following, followers) = tokio::try_join!(self.list_following(), self.list_followers())?;
    let follower_ids: HashSet<u64> = followers.iter().map(|u| u.id).collect();
    Ok(
      following
        .into_iter()
        .filter(|u| !follower_ids.contains(&u.id))
        .collect(),
    )
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

  fn rate_limit_error(&self, status: reqwest::StatusCode, context: &str) -> anyhow::Error {
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
      anyhow!(
        "API GitHub saturée (rate limit). {} : attendez un peu ou définissez GITHUB_TOKEN.",
        if self.token.is_none() {
          "Mode anonyme actif (60 req/h)"
        } else {
          "Token actif"
        }
      )
    } else {
      anyhow!("{context} (HTTP {})", status.as_u16())
    }
  }

  /// Recherche : GraphQL si token (1 requête/page), sinon REST N+1 budget-aware.
  pub async fn search_users(&self, criteria: &SearchCriteria) -> Result<SearchUsersResult> {
    self.wait_search_slot().await?;

    if self.token.is_some() {
      match self.search_graphql(criteria).await {
        Ok(result) => return Ok(result),
        Err(e) => log::warn!("GraphQL search failed ({e:#}); falling back to REST"),
      }
    }

    self.search_rest(criteria).await
  }

  async fn search_graphql(&self, criteria: &SearchCriteria) -> Result<SearchUsersResult> {
    let token = self
      .token
      .as_ref()
      .ok_or_else(|| anyhow!("token requis pour GraphQL"))?;

    let body = serde_json::json!({
      "query": GRAPHQL_QUERY,
      "variables": {
        "q": build_query(criteria),
        "first": criteria.per_page.unwrap_or(30).clamp(1, 100),
        "after": criteria.cursor,
      }
    });

    let resp = self
      .http
      .post(GRAPHQL_URL)
      .bearer_auth(token)
      .json(&body)
      .send()
      .await
      .context("requête GraphQL échouée")?;

    let status = resp.status();
    let remaining = resp
      .headers()
      .get("x-ratelimit-remaining")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.parse::<u32>().ok());

    let raw = resp.bytes().await.context("corps GraphQL illisible")?;
    if !status.is_success() {
      return Err(self.rate_limit_error(status, "requête GraphQL refusée"));
    }

    let payload: GqlResponse = serde_json::from_slice(&raw).context("réponse GraphQL invalide")?;

    if let Some(errors) = &payload.errors {
      let has_data = payload.data.as_ref().and_then(|d| d.search.as_ref()).is_some();
      if !has_data {
        let msg = errors
          .iter()
          .map(|e| e.message.clone())
          .collect::<Vec<_>>()
          .join("; ");
        return Err(anyhow!("GraphQL: {msg}"));
      }
    }

    let data = payload.data.ok_or_else(|| anyhow!("GraphQL: réponse vide"))?;
    let search = data
      .search
      .ok_or_else(|| anyhow!("GraphQL: champ search absent"))?;

    let mut users = Vec::with_capacity(search.edges.len());
    for edge in search.edges {
      match serde_json::from_value::<GqlUser>(edge.node) {
        Ok(u) => users.push(User {
          id: u.database_id.unwrap_or(0),
          login: u.login,
          avatar_url: u.avatar_url,
          html_url: u.url,
          name: u.name,
          bio: u.bio,
          location: u.location,
          company: u.company,
          created_at: u.created_at,
          public_repos: u.repositories.as_ref().map_or(0, |r| r.total_count),
          followers: u.followers.map_or(0, |f| f.total_count),
          following: u.following.map_or(0, |f| f.total_count),
          pushed_at: u
            .repositories
            .and_then(|r| r.nodes.into_iter().next())
            .and_then(|n| n.pushed_at),
        }),
        Err(_) => continue,
      }
    }

    Ok(SearchUsersResult {
      total_count: search.user_count,
      has_next: search.page_info.has_next_page,
      end_cursor: search.page_info.end_cursor,
      users,
      remaining,
      partial: false,
    })
  }

  async fn search_rest(&self, criteria: &SearchCriteria) -> Result<SearchUsersResult> {
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
      .map_err(|e| match e.status() {
        Some(status) => self.rate_limit_error(status, "erreur recherche"),
        None => anyhow!("erreur réseau : {e}"),
      })?;

    let search_remaining = resp
      .headers()
      .get("x-ratelimit-remaining")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.parse::<u32>().ok());

    let search: SearchResponse = resp.json().await.context("failed to parse search response")?;

    // Le quota pertinent pour les profils est le quota CORE, pas SEARCH.
    // GET /rate_limit est gratuit (ne consomme rien).
    let core_remaining = self.core_rate_limit_remaining().await;

    let budget = core_remaining
      .map(|r| r.saturating_sub(1) as usize)
      .unwrap_or(search.items.len())
      .min(search.items.len());

    let fetched = self.fetch_profiles(&search.items[..budget]).await;

    let mut users = Vec::with_capacity(search.items.len());
    for item in &search.items {
      if let Some(u) = fetched.get(&item.login) {
        users.push(u.clone());
      } else {
        users.push(basic_user(item));
      }
    }

    let partial = users.iter().any(|u| u.id == 0);
    let has_next = u64::from(page) * u64::from(per_page) < search.total_count && !users.is_empty();

    Ok(SearchUsersResult {
      total_count: search.total_count,
      users,
      remaining: search_remaining,
      partial,
      end_cursor: None,
      has_next,
    })
  }

  async fn core_rate_limit_remaining(&self) -> Option<u32> {
    let mut req = self.http.get(format!("{API_BASE}/rate_limit"));
    if let Some(token) = &self.token {
      req = req.bearer_auth(token);
    }
    let resp = req.send().await.ok()?;
    let rl: RateLimitResponse = resp.json().await.ok()?;
    Some(rl.resources.core.remaining)
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
      .map_err(|e| match e.status() {
        Some(status) => self.rate_limit_error(status, "erreur profil"),
        None => anyhow!("erreur réseau : {e}"),
      })?;
    let raw: RawUser = resp.json().await.context("failed to parse user response")?;
    Ok(raw.into())
  }

  /// Récupère les profils avec concurrence limitée et arrêt immédiat en cas
  /// de rate limit (évite les rate limits secondaires de GitHub).
  async fn fetch_profiles(&self, items: &[SearchItem]) -> HashMap<String, User> {
    let concurrency = if self.token.is_some() {
      CONCURRENCY_WITH_TOKEN
    } else {
      CONCURRENCY_ANONYMOUS
    };

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let aborted = Arc::new(AtomicBool::new(false));
    let http = self.http.clone();
    let token = self.token.clone();

    let mut tasks = tokio::task::JoinSet::new();
    for item in items {
      let login = item.login.clone();
      let http = http.clone();
      let token = token.clone();
      let sem = semaphore.clone();
      let aborted = aborted.clone();

      tasks.spawn(async move {
        if aborted.load(Ordering::Relaxed) {
          return (login, Err(ProfileError::Skipped));
        }
        let permit = sem.acquire_owned().await;
        if aborted.load(Ordering::Relaxed) {
          drop(permit.ok());
          return (login, Err(ProfileError::Skipped));
        }

        let mut req = http.get(format!("{API_BASE}/users/{login}"));
        if let Some(token) = &token {
          req = req.bearer_auth(token);
        }

        let result = match req.send().await {
          Ok(resp) if resp.status().is_success() => match resp.json::<RawUser>().await {
            Ok(raw) => Ok(raw.into()),
            Err(_) => Err(ProfileError::Other),
          },
          Ok(resp) if resp.status() == reqwest::StatusCode::FORBIDDEN => {
            aborted.store(true, Ordering::Relaxed);
            Err(ProfileError::RateLimited)
          }
          Ok(resp) if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
            aborted.store(true, Ordering::Relaxed);
            Err(ProfileError::RateLimited)
          }
          Ok(_) => Err(ProfileError::Other),
          Err(_) => Err(ProfileError::Other),
        };

        drop(permit);
        (login, result)
      });
    }

    let mut map = HashMap::with_capacity(items.len());
    while let Some(res) = tasks.join_next().await {
      if let Ok((login, Ok(user))) = res {
        map.insert(login, user);
      }
    }

    if aborted.load(Ordering::Relaxed) {
      log::warn!("fetch des profils interrompu : rate limit atteint");
    }

    map
  }
}

fn basic_user(item: &SearchItem) -> User {
  User {
    login: item.login.clone(),
    id: 0,
    avatar_url: item.avatar_url.clone(),
    html_url: item.html_url.clone(),
    name: None,
    bio: None,
    location: None,
    public_repos: 0,
    followers: 0,
    following: 0,
    pushed_at: None,
    created_at: None,
    company: None,
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

  parts.push("type:user".to_string());

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
  let valid =
    date.parse::<chrono::NaiveDate>().is_ok() || date.parse::<chrono::DateTime<chrono::Utc>>().is_ok();
  if !valid {
    return Err(anyhow!("invalid date format: {date} (expected YYYY-MM-DD)"));
  }
  Ok(())
}
