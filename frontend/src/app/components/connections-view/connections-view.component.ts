import { Component, effect, input, signal } from '@angular/core';
import { ConnectionTab, GithubService, SimpleUser } from '../../services/github.service';

@Component({
  selector: 'app-connections-view',
  standalone: true,
  imports: [],
  templateUrl: './connections-view.component.html',
  styleUrl: './connections-view.component.css',
})
export class ConnectionsViewComponent {
  readonly tab = input.required<ConnectionTab>();

  protected readonly loading = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly users = signal<SimpleUser[] | null>(null);
  protected readonly me = signal<SimpleUser | null>(null);

  constructor(readonly github: GithubService) {
    effect(() => void this.load(this.tab()));
  }

  private async load(tab: ConnectionTab): Promise<void> {
    this.loading.set(true);
    this.error.set(null);
    try {
      if (!this.me()) {
        this.me.set(await this.github.getMe());
      }
      const users =
        tab === 'followers'
          ? await this.github.getFollowers()
          : tab === 'following'
            ? await this.github.getFollowing()
            : await this.github.getNotFollowedBack();
      this.users.set(users);
    } catch (e) {
      this.error.set(String(e));
      this.users.set(null);
    } finally {
      this.loading.set(false);
    }
  }

  protected title(): string {
    switch (this.tab()) {
      case 'followers':
        return 'Followers';
      case 'following':
        return 'Following';
      case 'management':
        return 'Management';
    }
  }

  protected subtitle(): string | null {
    if (this.tab() === 'management') {
      return 'Personnes que vous suivez mais qui ne vous suivent pas en retour.';
    }
    return null;
  }
}
