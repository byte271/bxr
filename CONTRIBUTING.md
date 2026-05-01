# Contributing

BXR is early. Contributions should keep the project honest, testable, and browser-aware.

## Before Changing Code

- Read [`README.md`](README.md), [`docs/architecture.md`](docs/architecture.md), and the relevant spec in [`docs/specs`](docs/specs).
- Keep changes scoped to one subsystem when possible.
- Add or update tests for behavior changes.
- Do not expose a CPU feature, device, or browser capability before unsupported behavior traps clearly or is documented as unavailable.

## Quality Gate

Run:

```sh
npm run quality
```

This runs formatting, Rust tests, clippy, Wasm build, JavaScript syntax checks, and the Wasm smoke test.

## Documentation Rules

- Current support belongs in `README.md` and `docs/first-20-task-status.md`.
- Architecture boundaries belong in `docs/architecture.md`.
- Future capabilities belong in `docs/roadmap.md`.
- Versioned protocol/profile details belong in `docs/specs/`.
- Do not add duplicate roadmap reports or generated local state to the repository.
