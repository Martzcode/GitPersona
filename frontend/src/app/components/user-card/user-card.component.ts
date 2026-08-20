import { Component, input, output } from '@angular/core';
import { User } from '../../services/github.service';

@Component({
  selector: 'app-user-card',
  standalone: true,
  imports: [],
  templateUrl: './user-card.component.html',
  styleUrl: './user-card.component.css',
})
export class UserCardComponent {
  readonly user = input.required<User>();
  readonly view = output<User>();

  formatNumber(n: number): string {
    return new Intl.NumberFormat('fr-FR').format(n);
  }

  lastActivity(): string {
    if (!this.user().pushed_at) return 'Aucune activité publique';
    return new Date(this.user().pushed_at!).toLocaleDateString('fr-FR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }
}