# GitPersona

A desktop application for GitHub user discovery and follower/following relationship management.

## Why GitPersona?

- **Discover GitHub users worldwide** — search and filter accounts by location, repository count, follower/following ranges, and last activity date to find developers that match your criteria.
- **Track your followers and following** — monitor who follows you back and who doesn't, with a dedicated view that identifies users who follow you then unfollow after a follow-back, inflating their followers/following ratio. This lets you spot dishonest profiles and keep your network genuine.

## Features

- Advanced user search with filters (location, repos, followers, following, activity)
- Detailed profile view (bio, company, repos, activity, etc.)
- Follower and following management with one-click follow/unfollow
- Detection of users who don't follow you back
- Anonymous mode (low rate limits) or authenticated mode via GitHub token
- Rate-limit aware with real-time quota display
- GraphQL (authenticated) and REST (anonymous) dual search strategy

## Tech Stack

- **Tauri v2** — Rust backend + web frontend
- **Angular 22** — standalone, signals-based frontend
- **Rust** — `reqwest`, `tokio`, `serde`, `chrono`
- **GitHub API** — REST + GraphQL

## Getting Started

```bash
# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build
npm run tauri build
```

## Configuration

A GitHub Personal Access Token can be configured via the in-app settings dialog. Without a token, the app runs in anonymous mode with limited API rate limits.

## License

See [LICENSE](LICENSE).
