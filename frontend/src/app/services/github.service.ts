import { Injectable, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

export interface SearchCriteria {
  location?: string;
  min_repos?: number;
  max_repos?: number;
  min_followers?: number;
  max_followers?: number;
  min_following?: number;
  max_following?: number;
  last_activity_after?: string;
  per_page?: number;
  page?: number;
  cursor?: string;
}

export interface User {
  login: string;
  id: number;
  avatar_url: string;
  html_url: string;
  name: string | null;
  bio: string | null;
  location: string | null;
  public_repos: number;
  followers: number;
  following: number;
  pushed_at: string | null;
  created_at: string | null;
  company: string | null;
}

export interface SearchUsersResult {
  total_count: number;
  users: User[];
  remaining: number | null;
  partial: boolean;
  end_cursor: string | null;
  has_next: boolean;
}

export interface SimpleUser {
  login: string;
  id: number;
  avatar_url: string;
  html_url: string;
}

export type ConnectionTab = 'followers' | 'following' | 'management';

export interface AppConfig {
  github_username: string;
  token: string;
}

@Injectable({ providedIn: 'root' })
export class GithubService {
  readonly authenticated = signal(false);
  readonly checkingAuth = signal(true);

  constructor() {
    this.checkAuth();
  }

  async checkAuth(): Promise<void> {
    try {
      this.authenticated.set(await invoke<boolean>('is_authenticated'));
    } catch {
      this.authenticated.set(false);
    } finally {
      this.checkingAuth.set(false);
    }
  }

  async searchUsers(criteria: SearchCriteria): Promise<SearchUsersResult> {
    return invoke<SearchUsersResult>('search_users', { criteria });
  }

  async getUser(login: string): Promise<User> {
    return invoke<User>('get_user', { login });
  }

  private meCache: SimpleUser | null = null;

  async getMe(): Promise<SimpleUser> {
    if (!this.meCache) {
      this.meCache = await invoke<SimpleUser>('get_me');
    }
    return this.meCache;
  }

  async getFollowers(): Promise<SimpleUser[]> {
    return this.cached('followers', () => invoke<SimpleUser[]>('get_followers'));
  }

  async getFollowing(): Promise<SimpleUser[]> {
    return this.cached('following', () => invoke<SimpleUser[]>('get_following'));
  }

  async getNotFollowedBack(): Promise<SimpleUser[]> {
    return this.cached('not_followed_back', () =>
      invoke<SimpleUser[]>('get_not_followed_back'),
    );
  }

  clearConnectionsCache(): void {
    this.connectionsCache.clear();
    this.meCache = null;
  }

  async getConfig(): Promise<AppConfig> {
    return invoke<AppConfig>('get_config');
  }

  async saveConfig(config: AppConfig): Promise<void> {
    await invoke('save_config', { config });
    this.clearConnectionsCache();
    await this.checkAuth();
  }

  private readonly connectionsCache = new Map<string, SimpleUser[]>();

  private async cached(
    key: string,
    loader: () => Promise<SimpleUser[]>,
  ): Promise<SimpleUser[]> {
    if (!this.connectionsCache.has(key)) {
      try {
        this.connectionsCache.set(key, await loader());
      } catch (e) {
        this.connectionsCache.delete(key);
        throw e;
      }
    }
    return this.connectionsCache.get(key)!;
  }
}