import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { invoke } from '@tauri-apps/api/core';

@Component({
  imports: [FormsModule],
  selector: 'app-root',
  styleUrl: './app.css',
  templateUrl: './app.html',
})
export class App {
  protected readonly title = signal('GitPersona');
  protected readonly message = signal('Clique pour appeler Rust');
  protected readonly name = signal('GitPersona');

  async greet(): Promise<void> {
    this.message.set(await invoke<string>('greet', { name: this.name() }));
  }
}