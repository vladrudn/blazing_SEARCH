# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Rust backend. Key modules include indexing, search, and web serving (see `src/main.rs`, `src/search_engine.rs`, `src/web_server.rs`).
- `web/` holds static frontend assets (HTML/CSS/JS and images) served by the backend.
- Root scripts provide maintenance utilities (e.g., `rebuild_index.py`, `rebuild_inverted.rs`) and container entry points (`build-and-run.sh`).
- Docker assets live in `Dockerfile` and `docker-compose.yml`.

## Build, Test, and Development Commands
- `cargo build` — compile the Rust service locally.
- `cargo run` — start the service locally (expects configuration and data paths to be available).
- `cargo test` — run unit tests (currently minimal).
- `./build-and-run.sh` — build and start via Docker Compose (wrapper for `docker-compose up --build -d`).
- `docker-compose logs -f` — follow container logs during runtime.

## Coding Style & Naming Conventions
- Rust code follows standard `rustfmt` defaults (4-space indent, snake_case for functions/vars, CamelCase for types).
- Frontend files in `web/` use conventional JS/CSS naming; keep style consistent with nearby files.
- Prefer clear, descriptive names for index-related data (`documents_index.json`, `inverted_index.json`).

## Testing Guidelines
- Tests use Rust’s built-in test framework (`#[cfg(test)]`); see `src/stemmer.rs` for examples.
- Add unit tests for text normalization or indexing behavior when changing search logic.
- Run `cargo test` before submitting changes.

## Commit & Pull Request Guidelines
- Commit messages in this repo are free-form and descriptive, often in Ukrainian and sometimes date-prefixed. Follow the same style (short summary of the change; add context if helpful).
- PRs should include: a brief summary, testing notes (commands run), and screenshots for UI changes in `web/`.

## Configuration & Runtime Notes
- The app relies on external document directories and cache/index files. Docker setups typically mount these via `docker-compose.yml`.
- Index rebuild helpers (`rebuild_index.py`) assume `documents_index.json` exists in the repo root.
