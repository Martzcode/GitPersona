import { Component, output } from '@angular/core';

@Component({
  selector: 'app-token-permissions-dialog',
  standalone: true,
  imports: [],
  templateUrl: './token-permissions-dialog.component.html',
  styleUrl: './token-permissions-dialog.component.css',
})
export class TokenPermissionsDialogComponent {
  readonly close = output<void>();
}
