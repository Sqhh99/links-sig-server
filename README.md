# links-sig-server

The backend for [Links](https://github.com/Sqhh99/links), a Qt desktop client for audio/video meetings. It is a Rust service built on [Axum](https://github.com/tokio-rs/axum), with PostgreSQL for storage and a [LiveKit](https://livekit.io) server for media.

The desktop client talks to this service over a small REST API. This service then issues LiveKit access tokens, and the client connects to LiveKit directly with them.

## Features

- **Accounts**
  - Log in with a username and password. There is no sign-up step or email verification: the first login with a new username creates the account.
  - New usernames are 2–32 characters: letters, digits, `_`, `-` or `.`. They are unique regardless of case.
  - New passwords must be 8–128 characters long, contain at least one letter and one digit, and differ from the username.
  - Sessions use JWTs that can be refreshed.
- **Meetings**
  - Every meeting gets a 9-digit meeting number, and can be started right away or scheduled.
  - The host has to join before anyone else can.
  - Optional features:
    - a meeting password;
    - guest access, which lets people join without an account.
- **Meeting lifecycle**
  - A background worker ends meetings that nobody joined, or that stayed empty for too long.
  - Records of joined and hosted meetings are stored per user.
- **LiveKit**
  - Issues access tokens.
  - Lists, creates and deletes rooms.
  - Lists and removes participants.
- **Web join page**
  - `/join?meetingNo=…` is a browser page that lets a guest watch a meeting from its share link without installing the client.
  - It only works when the meeting allows guests. Guests can watch and listen, but not publish.

## Quick start with Docker

[`docker/compose.yaml`](docker/compose.yaml) starts PostgreSQL, LiveKit and this server with one command:

```bash
cd docker
cp .env.example .env    # fill in JWT_SECRET and LIVEKIT_API_SECRET (openssl rand -hex 32)
docker compose up -d --build
curl http://127.0.0.1:8081/api/health
```

Then point the Links client at `http://127.0.0.1:8081`.

[`docker/README.md`](docker/README.md) is the full guide (in Chinese). It covers:

- logs and day-to-day commands;
- every setting in `.env`;
- making LiveKit reachable from other machines on the LAN or the internet;
- migrating from the old Docker setup.

## Running from source

You need a recent stable Rust toolchain (tested with 1.98, which the Docker image also uses), plus PostgreSQL and LiveKit. The easiest way to get PostgreSQL and LiveKit is to run just those two services from the Docker setup:

```bash
cd docker && docker compose up -d postgres livekit && cd ..
cargo run
```

Configuration is read from environment variables, and from a `.env` file in the working directory if one exists. The `.env` must use the same `LIVEKIT_API_KEY` and `LIVEKIT_API_SECRET` as `docker/.env`.

The server applies the database migrations in [`migrations/`](migrations) automatically at startup.

| Variable | Default | Purpose |
| --- | --- | --- |
| `SERVER_PORT` | `8081` | HTTP port. The server always binds `0.0.0.0` |
| `DATABASE_URL` | `postgres://links_sig:links_sig_password@localhost:5432/links_sig` | PostgreSQL connection string |
| `LIVEKIT_URL` | `http://localhost:7880` | LiveKit API address, as seen from this server |
| `LIVEKIT_WS_URL` | `ws://127.0.0.1:7880` | LiveKit address handed to clients. It must be reachable from them |
| `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET` | `devkey` / `secret` | Must match the keys configured in LiveKit |
| `JWT_SECRET` | a placeholder | Signs user tokens. **Always set this** |
| `JWT_EXPIRATION_SECS` | `604800` (7 days) | User token lifetime |
| `APP_BASE_URL` | `http://localhost:3000` | Prefix for meeting share links; usually this server's own public URL |
| `MEETING_LIFECYCLE_INTERVAL_SECS` | `60` | How often the lifecycle worker runs |
| `ENABLE_HTTPS`, `SSL_CERT_FILE`, `SSL_KEY_FILE` | `false`, `./certs/server.crt`, `./certs/server.key` | Serve HTTPS directly instead of behind a reverse proxy |

## Tests

The integration tests need a separate PostgreSQL database on port 5433. It is defined in the same compose file, but only starts when you name it:

```bash
docker compose -f docker/compose.yaml up -d postgres-test
cargo test
```

Set `TEST_DATABASE_URL` to use a different test database. Run `cargo fmt` before sending changes.

## API

Every endpoint lives under `/api`. The reference docs, in Chinese, are in [`docs/api/`](docs/api/README.md).

| Area | Endpoints |
| --- | --- |
| Health | `GET /api/health` |
| Auth | `POST /api/auth/login`, `POST /api/auth/refresh` |
| Meetings | `POST /api/meetings`; `POST /api/meetings/{meeting_no}/join`, `/guest-join`, `/leave`, `/cancel` |
| Current user | `GET /api/me/meeting-records`, `GET /api/me/host-meetings` |
| LiveKit | `POST /api/token`; `GET` / `POST /api/rooms`; `DELETE /api/rooms/{room}`; `GET /api/rooms/{room}/participants`; `DELETE /api/rooms/{room}/participants/{identity}`; `POST /api/rooms/{room}/end` |

## Project layout

```
src/
  main.rs            startup: config, database pool, migrations, lifecycle worker, HTTP server
  config.rs          environment-based configuration
  routes.rs          all API routes in one place
  handlers/          thin HTTP handlers
  services/          business logic: login, meetings, meeting lifecycle, LiveKit tokens
  integrations/      PostgreSQL pool and LiveKit API client
  auth/              JWT encoding and the Axum extractors for authenticated requests
  types/             request/response DTOs and the AppError type
migrations/          SQL migrations, applied at startup
static/              the /join web page (vanilla JS plus livekit-client)
tests/               integration tests; tests/support/ has the fake LiveKit and DB helpers
docker/              compose file, LiveKit config and .env template
docs/                API reference, contributing guide, troubleshooting
```

Handlers stay thin and put their logic in services. The LiveKit client sits behind a trait, so the tests use a fake LiveKit and a real test database. The conventions are in [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md), and common problems are covered in [`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md). Both are in Chinese.
