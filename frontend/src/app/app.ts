import { Component, signal } from '@angular/core';
import { TitleBarComponent } from './components/title-bar/title-bar.component';
import { SidebarComponent, ViewId } from './components/sidebar/sidebar.component';
import { SearchViewComponent } from './components/search-view/search-view.component';
import { ConnectionsViewComponent } from './components/connections-view/connections-view.component';

@Component({
  selector: 'app-root',
  imports: [TitleBarComponent, SidebarComponent, SearchViewComponent, ConnectionsViewComponent],
  styleUrl: './app.css',
  templateUrl: './app.html',
})
export class App {
  protected readonly activeView = signal<ViewId>('search');

  protected onNavigate(view: ViewId): void {
    this.activeView.set(view);
  }
}
