import { Component, output, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { AppConfig, GithubService } from '../../services/github.service';

@Component({
  selector: 'app-settings-dialog',
  standalone: true,
  imports: [FormsModule],
  templateUrl: './settings-dialog.component.html',
  styleUrl: './settings-dialog.component.css',
})
export class SettingsDialogComponent {
  readonly close = output<void>();

  protected readonly username = signal('');
  protected readonly token = signal('');
  protected readonly showToken = signal(false);
  protected readonly loading = signal(true);
  protected readonly saving = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly saved = signal(false);

  constructor(private readonly github: GithubService) {
    void this.init();
  }

  private async init(): Promise<void> {
    try {
      const config = await this.github.getConfig();
      this.username.set(config.github_username);
      this.token.set(config.token);
    } catch (e) {
      this.error.set(String(e));
    } finally {
      this.loading.set(false);
    }
  }

  protected async save(): Promise<void> {
    this.saving.set(true);
    this.error.set(null);
    const config: AppConfig = {
      github_username: this.username().trim(),
      token: this.token().trim(),
    };
    try {
      await this.github.saveConfig(config);
      this.saved.set(true);
      setTimeout(() => this.close.emit(), 600);
    } catch (e) {
      this.error.set(String(e));
    } finally {
      this.saving.set(false);
    }
  }
}
