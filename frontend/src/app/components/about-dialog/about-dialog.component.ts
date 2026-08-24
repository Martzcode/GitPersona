import { Component, output, signal } from '@angular/core';
import { getVersion } from '@tauri-apps/api/app';

@Component({
  selector: 'app-about-dialog',
  standalone: true,
  imports: [],
  templateUrl: './about-dialog.component.html',
  styleUrl: './about-dialog.component.css',
})
export class AboutDialogComponent {
  readonly close = output<void>();

  protected readonly version = signal('');

  constructor() {
    getVersion()
      .then(v => this.version.set(v))
      .catch(() => this.version.set(''));
  }
}
