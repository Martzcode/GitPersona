import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { UserCardComponent } from '../user-card/user-card.component';
import { UserProfileComponent } from '../user-profile/user-profile.component';
import { GithubService, SearchCriteria, SearchUsersResult, User } from '../../services/github.service';

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
  selector: 'app-search-view',
  standalone: true,
  imports: [FormsModule, UserCardComponent, UserProfileComponent],
  templateUrl: './search-view.component.html',
  styleUrl: './search-view.component.css',
})
export class SearchViewComponent {
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
  protected readonly selectedLogin = signal<string | null>(null);
  private readonly cursors: (string | undefined)[] = [];

  constructor(readonly github: GithubService) {}

  protected async search(resetPage = true): Promise<void> {
    if (resetPage) {
      this.page.set(1);
      this.cursors.length = 0;
    }
    this.loading.set(true);
    this.error.set(null);
    this.selectedLogin.set(null);

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
      cursor: this.page() > 1 ? this.cursors[this.page() - 2] : undefined,
    };

    try {
      const result = await this.github.searchUsers(criteria);
      if (result.end_cursor) {
        this.cursors[this.page() - 1] = result.end_cursor;
      }
      this.result.set(result);
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

  protected viewUser(user: User): void {
    this.selectedLogin.set(user.login);
  }

  protected closeDetail(): void {
    this.selectedLogin.set(null);
  }

  protected formatNumber(n: number): string {
    return new Intl.NumberFormat('fr-FR').format(n);
  }
}
