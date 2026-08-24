import { Component, signal } from '@angular/core';
import { TitleBarComponent } from './components/title-bar/title-bar.component';
import { SidebarComponent, ViewId } from './components/sidebar/sidebar.component';
import { SearchViewComponent } from './components/search-view/search-view.component';
import { ConnectionsViewComponent } from './components/connections-view/connections-view.component';
import { SettingsDialogComponent } from './components/settings-dialog/settings-dialog.component';
import { AboutDialogComponent } from './components/about-dialog/about-dialog.component';
import { TokenPermissionsDialogComponent } from './components/token-permissions-dialog/token-permissions-dialog.component';

@Component({
  selector: 'app-root',
  imports: [TitleBarComponent, SidebarComponent, SearchViewComponent, ConnectionsViewComponent, SettingsDialogComponent, AboutDialogComponent, TokenPermissionsDialogComponent],
  styleUrl: './app.css',
  templateUrl: './app.html',
})
export class App {
  protected readonly activeView = signal<ViewId>('search');
  protected readonly settingsOpen = signal(false);
  protected readonly aboutOpen = signal(false);
  protected readonly tokenPermissionsOpen = signal(false);

  protected onNavigate(view: ViewId): void {
    this.activeView.set(view);
  }
}
