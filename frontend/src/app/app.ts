import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TitleBarComponent } from './components/title-bar/title-bar.component';
import { UserCardComponent } from './components/user-card/user-card.component';
import { GithubService, SearchCriteria, SearchUsersResult, User } from './services/github.service';

interface CriteriaForm {
  location: string;
  min_repos: number | null;
  max_repos: number | null;
  min_followers: number | null;
  max_followers: number | null;
  min_following: number | null;
  max_following: number | null;
  last_activity_after: string;
}

@Component({
  selector: 'app-root',
  imports: [FormsModule, TitleBarComponent, UserCardComponent],
  styleUrl: './app.css',
  templateUrl: './app.html',
})
export class App {
  protected readonly form: CriteriaForm = {
    location: '',
    min_repos: null,
    max_repos: null,
    min_followers: null,
    max_followers: null,
    min_following: null,
    max_following: null,
    last_activity_after: '',
  };

  protected readonly loading = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly result = signal<SearchUsersResult | null>(null);
  protected readonly page = signal(1);
  protected readonly selectedUser = signal<User | null>(null);
  protected readonly detailLoading = signal(false);

  constructor(readonly github: GithubService) {}

  protected async search(resetPage = true): Promise<void> {
    if (resetPage) this.page.set(1);
    this.loading.set(true);
    this.error.set(null);
    this.selectedUser.set(null);

    if (this.form.last_activity_after && !/^\d{4}-\d{2}-\d{2}$/.test(this.form.last_activity_after)) {
      this.error.set('Format de date invalide. Utilise AAAA-MM-JJ (ex : 2025-01-01).');
      this.loading.set(false);
      return;
    }

    const criteria: SearchCriteria = {
      location: this.form.location || undefined,
      min_repos: this.form.min_repos ?? undefined,
      max_repos: this.form.max_repos ?? undefined,
      min_followers: this.form.min_followers ?? undefined,
      max_followers: this.form.max_followers ?? undefined,
      min_following: this.form.min_following ?? undefined,
      last_activity_after: this.form.last_activity_after || undefined,
      per_page: 30,
      page: this.page(),
    };

    try {
      this.result.set(await this.github.searchUsers(criteria));
    } catch (e) {
      this.error.set(String(e));
    } finally {
      this.loading.set(false);
    }
  }

  protected async nextPage(): Promise<void> {
    this.page.update(p => p + 1);
    await this.search(false);
  }

  protected async previousPage(): Promise<void> {
    if (this.page() <= 1) return;
    this.page.update(p => p - 1);
    await this.search(false);
  }

  protected async viewUser(user: User): Promise<void> {
    this.selectedUser.set(user);
    this.detailLoading.set(true);
    try {
      this.selectedUser.set(await this.github.getUser(user.login));
    } catch (e) {
      this.error.set(String(e));
    } finally {
      this.detailLoading.set(false);
    }
  }

  protected closeDetail(): void {
    this.selectedUser.set(null);
  }

  protected formatNumber(n: number): string {
    return new Intl.NumberFormat('fr-FR').format(n);
  }

  protected formatDate(date: string | null): string {
    if (!date) return '—';
    return new Date(date).toLocaleDateString('fr-FR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }
}