# Repository Guidelines

## Project Structure & Module Organization
- The repository currently has only `LICENSE`; add application code in `src/` with a single entrypoint (for example, `src/main.*` or `src/index.*`) and shared utilities in `src/lib/` or `src/common/`.
- Mirror the structure in `tests/` and keep fixtures under `tests/fixtures/` for deterministic inputs.
- Use `docs/` for design notes, `assets/` for static files, and `scripts/` for tooling or one-off migrations.

## Build, Test, and Development Commands
- Keep a `Makefile` (or similar) as the single entrypoint. Recommended targets: `make setup` (install deps), `make dev` (run local app with reload), `make test` (all suites), `make lint` / `make fmt` (style enforcement), `make build` (production artifact), `make clean` (remove build outputs).
- If containers are used, support `docker compose up --build` as the one-step bootstrap and document required env vars in `.env.example`.

## Coding Style & Naming Conventions
- Enforce language-native formatters (`prettier`, `black`, `gofmt`, `rustfmt`, etc.) via `make fmt`; never hand-format.
- Favor small modules with clear boundaries; avoid cross-import cycles between `src/` subpackages.
- Naming: PascalCase for classes/types, camelCase for functions and variables, kebab-case for directories, snake_case for standalone scripts/config.
- Keep configuration in `.env` (never commit secrets) and check in `.env.example` listing required keys.

## Testing Guidelines
- Mirror `src/` layout in `tests/`; name suites `*.spec.*`, `*.test.*`, or `test_*.py` depending on language conventions.
- Target ≥80% coverage for core areas and add a focused regression test for each bug fix.
- Prefer deterministic tests with fixtures and mocks; use `make test` locally and ensure CI executes the same command (document any long-running integration suites separately).

## Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`) in imperative mood; keep commits small and reversible.
- PRs should include a concise summary, linked issues, test evidence (`make test` output or UI screenshots), and rollout/rollback notes when relevant.
- Keep scope tight; prefer follow-up issues for non-critical additions.

## Security & Configuration Tips
- Do not commit secrets or certificates; keep `.env` and build outputs ignored.
- Run dependency health checks (`npm audit`, `pip audit`, `cargo audit`, etc.) during `make setup` or CI.
- Review third-party licenses before adding dependencies and document exceptions in `docs/` when necessary.
