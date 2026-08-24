import { Component, OnInit, output, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

@Component({
  selector: 'app-title-bar',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './title-bar.component.html',
  styleUrl: './title-bar.component.css',
})
export class TitleBarComponent implements OnInit {
  readonly settingsRequested = output<void>();
  readonly aboutRequested = output<void>();

  protected readonly appName = signal('GitPersona');
  protected readonly isMaximized = signal(false);
  protected readonly menus = signal<MenuItem[]>([
    { label: 'Fichier', items: [{ label: 'Nouveau', action: () => console.log('Nouveau') }, { label: 'Ouvrir', action: () => console.log('Ouvrir') }, { type: 'separator' }, { label: 'Quitter', action: () => this.close() }] },
    { label: 'Édition', items: [{ label: 'Annuler', action: () => console.log('Annuler') }, { label: 'Rétablir', action: () => console.log('Rétablir') }] },
    { label: 'Affichage', items: [{ label: 'Plein écran', action: () => this.toggleMaximize() }, { label: 'Redimensionner', action: () => console.log('Redimensionner') }] },
    { label: 'Paramètres', action: () => this.settingsRequested.emit() },
    { label: 'Aide', items: [{ label: 'À propos', action: () => this.aboutRequested.emit() }] },
  ]);

  private window = getCurrentWebviewWindow();

  ngOnInit(): void {
    this.window.onResized(() => {
      this.checkMaximized();
    });
    this.window.listen('tauri://resize', () => {
      this.checkMaximized();
    });
  }

  protected showAppMenu(event: MouseEvent): void {
    event.stopPropagation();
    console.log('App menu clicked');
  }

  protected toggleMenu(event: Event, menu: MenuItem): void {
    event.stopPropagation();
    if (!menu.items || menu.items.length === 0) {
      menu.action?.();
      this.menus.update(menus => menus.map(m => ({ ...m, open: false })));
      return;
    }
    menu.open = !menu.open;
    this.menus.update(menus => menus.map(m => m === menu ? { ...m, open: menu.open } : { ...m, open: false }));
  }

  private async checkMaximized(): Promise<void> {
    this.isMaximized.set(await this.window.isMaximized());
  }

  protected async minimize(): Promise<void> {
    await this.window.minimize();
  }

  protected async toggleMaximize(): Promise<void> {
    if (await this.window.isMaximized()) {
      await this.window.unmaximize();
    } else {
      await this.window.maximize();
    }
    this.checkMaximized();
  }

  protected async close(): Promise<void> {
    await this.window.close();
  }

  protected onMenuItemClick(item: MenuItem): void {
    item.action?.();
    this.menus.update(menus => menus.map(m => ({ ...m, open: false })));
  }
}

interface MenuItem {
  label?: string;
  items?: MenuItem[];
  action?: () => void;
  type?: 'separator';
  open?: boolean;
}