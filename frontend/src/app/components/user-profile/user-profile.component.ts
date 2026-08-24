import { Component, effect, input, output, signal } from '@angular/core';
import { GithubService, User } from '../../services/github.service';

@Component({
  selector: 'app-user-profile',
  standalone: true,
  imports: [],
  templateUrl: './user-profile.component.html',
  styleUrl: './user-profile.component.css',
})
export class UserProfileComponent {
  readonly login = input.required<string>();
  readonly avatarUrl = input('');
  readonly close = output<void>();

  protected readonly loading = signal(true);
  protected readonly error = signal<string | null>(null);
  protected readonly profile = signal<User | null>(null);

  constructor(private readonly github: GithubService) {
    effect(() => void this.load(this.login()));
  }

  private async load(login: string): Promise<void> {
    this.loading.set(true);
    this.error.set(null);
    this.profile.set(null);
    try {
      this.profile.set(await this.github.getUser(login));
    } catch (e) {
      this.error.set(String(e));
    } finally {
      this.loading.set(false);
    }
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
