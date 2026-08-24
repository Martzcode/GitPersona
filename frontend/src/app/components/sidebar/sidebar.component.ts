import { Component, input, output, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ConnectionTab } from '../../services/github.service';

export type ViewId = 'search' | ConnectionTab;

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.css',
})
export class SidebarComponent {
  readonly activeView = input.required<ViewId>();
  readonly navigate = output<ViewId>();

  protected readonly connectionsOpen = signal(false);

  protected isConnectionsActive(): boolean {
    return this.activeView() !== 'search';
  }

  protected selectSearch(event: Event): void {
    event.stopPropagation();
    this.connectionsOpen.set(false);
    this.navigate.emit('search');
  }

  protected toggleConnections(event: Event): void {
    event.stopPropagation();
    if (!this.isConnectionsActive()) {
      this.navigate.emit('followers');
      this.connectionsOpen.set(true);
      return;
    }
    this.connectionsOpen.update(open => !open);
  }

  protected selectTab(tab: ConnectionTab): void {
    this.connectionsOpen.set(false);
    this.navigate.emit(tab);
  }
}
