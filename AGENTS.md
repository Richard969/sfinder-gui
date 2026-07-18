# Repository Guidelines

**sfinder-gui** — Tauri v2 desktop GUI for [solution-finder](https://github.com/knewjade/solution-finder), a Tetris perfect-clear solver. User draws/pastes a Tetris field, picks a command (percent/path/setup/ren/spin/cover), runs a Java JAR (solution-finder), and views parsed results. Includes screen-capture OCR to import fields from a live game (tetr.io/jstris).

Stack: **Tauri v2 + React 19 + TypeScript 6 + Vite 8 + Tailwind CSS v4 + Zustand 5 + shadcn/ui**. Rust backend handles IPC, process spawning, and image recognition.

## Project Overview

- **Purpose**: Desktop GUI wrapper around the CLI tool `solution-finder`. Provides a visual fumen editor and command runner for common Tetris solving workflows (percent/PC-probability, path/solutions, setup, ren, spin, cover).
- **Current status**: **Percent**, **Path**, and **Spin** are wired. Setup, Ren, and Cover are WIP placeholders.
- **Two runtimes**: A browser-based frontend (Vite dev server) and a Tauri desktop app (frontend + Rust backend). Use Tauri dev mode for full functionality (Java spawn, file OCR).
- **Local-first**: All computation runs locally via a bundled or user-supplied Java JAR.

## Architecture & Data Flow

```
React (UI)
   │  invoke("run_sfinder_command", config)
  ▼
Rust #[tauri::command] (src-tauri/src/commands.rs)
  │  spawn: java -jar sfinder.jar <command> <args>
  │  stream stdout/stderr via Tokio channels
  ▼
Capture output files (.html, .csv, .txt) from CWD
  ▼
Return SfinderOutput { status, stdout, stderr, output_files }
  │  app.emit("sfinder-events", ...) for streaming updates
  ▼
Frontend parses: output-parser.ts (parseSolutions, parsePercent, parseCoverage)
  ▼
Render: OutputViewer + PercentDisplay + SolutionTable (DOMPurified HTML)
```

**Frontend state** is split across three Zustand stores — none persist except `appStore` (localStorage key `sfinder-gui-settings`):

- `appStore.ts` — settings (javaPath, jarPath, theme, language, outputDir) + Java/JAR detection status.
- `fumenStore.ts` — multi-page fumen editor: field grid, palette tool, patterns, undo/redo (snapshot-based). Shared across all command pages.
- `commandStore.ts` — active command status (`idle | running | success | error | cancelled`) + history (last 50).

**Rust backend** (`src-tauri/src/`):
- `lib.rs` — registers 13 commands + 3 Tauri plugins (shell, fs, dialog).
- `commands.rs` — thin IPC handlers; sfinder runner, Java/JAR detection, screen-capture entry points.
- `sfinder.rs` — JVM spawn engine: `build_cli_args`, execute + stream + post-process CSV (path/cover aggregation, greedy set-cover for minimal solutions).
- `recognition.rs` — monitor capture (`screenshots` crate) + pixel-based Tetris OCR: palette detection (HSL + YUV), board region detection, cell-grid sampling → field string. Stores in a global `CaptureStore`.
- `color_split.rs` — flood-fill field → identify pieces by shape → topological sort (dependency graph on occupied cells) => ordered piece-operations for the Cover command.
- `kick_table.rs` — parses `.properties` kick-table files; includes built-in SRS table.

## sfinder CLI Reference

The GUI wraps a Java CLI tool. Full invocation:

```bash
java -jar sfinder.jar <command> [options]
java -jar sfinder.jar <command> -h   # list options for a command
```

Docs: https://solution-finder.readthedocs.io/ (Japanese; mirrors the JAR's built-in `--help`).

**Commands**

| Command | What it does | Status in GUI |
|---------|--------------|---------------|
| `percent` | PC success probability from a field | ✅ wired |
| `path` | Enumerate all PC solutions | ✅ wired |
| `setup` | Fill a field to a target shape | 🚧 WIP |
| `ren` | Continue a REN combo | 🚧 WIP |
| `spin` | Find T-spin setups | ✅ wired |
| `cover` | Probability of placing pieces per a given orientation set | 🚧 WIP (auto-split in Rust) |
| `util fig` / `util fumen` / `util seq` | Fumen → image / convert fumen / expand patterns | — |
| `verify kicks` | Validate a kick table | — |

**Common options** (`percent`/`path`/`setup`/`ren`/`spin`/`cover`)

| Flag | Default | Meaning |
|------|---------|---------|
| `-t` `--tetfu` | — | v115@... fumen code |
| `-P` `--page` | 1 | Page to load from the fumen |
| `-p` `--patterns` | — | Piece sequence to search (pattern syntax below) |
| `-H` `--hold` | `use` | `use` or `--hold avoid` |
| `-c` `--clear-line` | 4 | Lines to clear |
| `-K` `--kicks` | `srs` | Rotation system (`srs`, `@file`, `+file`, path) |
| `-d` `--drop` | `softdrop` | `softdrop` / `harddrop` / `180` / `t-softdrop` |
| `-f` `--format` | `path`: html | `path` output: `html` or `csv` |
| `-s` `--split` | `no` | `path`: split by orientation |

**Pattern syntax** (for `--patterns`)

- `*` = all 7 pieces `[TIJLSZO]`; `*p7` = all 5040 permutations (7!).
- `*p4` = 7P4 = 840 sequences; `I,*p4` = hold I then draw 4.
- `[SZLJ]p2` = 4P2 = 12 ordered picks from the bracket.
- `[SZLJ]!` = use all bracketed pieces (same as `[SZLJ]p4`).
- `[^TI]` = any piece except T, I.
- Comma joins elements (multiplied); semicolon in fumen comments separates distinct pattern groups: `--patterns T,*;I,*`.

**Input methods** (option priority: CLI > fumen comment > file defaults)

- CLI args (used by the GUI).
- Fumen comment field: encode options after the line-count, e.g. `4 --patterns *p4 --hold avoid`.
- Text files: `input/field.txt` (fumen or `X`/`_` grid) and `input/patterns.txt` (one pattern per line).

**Output**

- `path` writes `output/path.html` (default) or `.csv`; percent writes `output/last_output.txt`.
- Result files are read back by Rust and returned as `SfinderOutput.output_files`.
- `cover` emits CSV: rows = piece sequences, columns = orientation fumen, `O`/`X` = placeable/not; summary lines give OR/AND coverage.

When adding flags, check `sfinder.jar <command> -h` for the authoritative list — `src-tauri/src/sfinder.rs` `build_cli_args()` must match it.

## Key Directories

| Path | Purpose |
|------|---------|
| `src/main.tsx` | Frontend entry — React 19 mount + `BrowserRouter` |
| `src/App.tsx` | Root: route table (9 pages), startup Java/JAR detection, theme apply |
| `src/routes/` | One page per command + Home / FumenEditor / Settings / ViewFumen |
| `src/stores/` | Zustand stores (`appStore`, `fumenStore`, `commandStore`) |
| `src/components/fumen/` | Visual fumen editor — `FieldGrid` (10×23 interactive grid), `FieldCell`, `PiecePalette`, `FumenToolbar`, `FumenEditorEmbed`, `PageNavigator` |
| `src/components/forms/` | Shared forms — `PatternInput`, `CommandOptions` (hold/drop/kicks/split), `CommandRunner` (execute/cancel) |
| `src/components/output/` | Result rendering — `OutputViewer`, `PercentDisplay` (SVG ring), `SolutionTable`, `RawOutput`, `SanitizedHtml` |
| `src/components/layout/` | `AppLayout` (sidebar + content), `Sidebar`, `Header` |
| `src/lib/` | `sfinder-args.ts` (frontend CLI-arg mirror), `output-parser.ts`, `sanitize-html.ts` (DOMPurify) |
| `src/hooks/` | `useSfinderCommand.ts` (merge settings → invoke Rust → update commandStore), `usePieceKeys.ts` |
| `src/i18n/` | `translations.ts` (en + zh), `useTranslation.ts` |
| `src/types/` | `app.ts`, `sfinder.ts` (`SfinderCommand`, `CommandStatus`, `SfinderOutput`, `PathResultEntry`, `CoverResultEntry`), `fumen.ts` |
| `src-tauri/src/` | Rust backend (commands, sfinder runner, OCR, color-split auto-split, kick table) |
| `src-tauri/tests/` | Rust integration tests (board recognition against PNG fixtures) |
| `scripts/bundle-jre.js` | Bundles a minimal JRE (~35–40 MB) into the Tauri app via `jlink` |
| `overlay.html` | Full-screen transparent overlay for capture-region selection |
| `.github/workflows/` | CI (test + build matrix), release (sign + GitHub Release upload), rust-tests |

## Development Commands

```bash
# Install (pnpm recommended — pnpm-lock.yaml present; npm also works)
npm install

# Frontend-only dev server (http://localhost:1420) — no Java/JAR/ocr
npm run dev

# Full Tauri dev mode (frontend + Rust backend + Java spawn)  <-- primary
npm run tauri dev

# TypeScript type check
npx tsc --noEmit        # or: npm run build (tsc -b && vite build)

# Rust type check
cd src-tauri && cargo check

# Production build (outputs to src-tauri/target/release/bundle/)
npm run tauri build

# Tests
npm run test            # vitest run (TypeScript — declared, 0 test files exist)
cd src-tauri && cargo test          # Rust integration + unit tests
cd src-tauri && cargo test --test-threads=1   # CI default for recognition tests
```

## Code Conventions & Common Patterns

**Naming**
- Files: kebab-case (`fumen-store.ts`, `output-viewer.tsx`).
- Components & types: PascalCase without `I` prefix (`FieldGrid`, `CommandStatus`).
- Variables/functions: camelCase.
- Rust: `snake_case`, with `#[serde(rename_all = "camelCase")]` for the JS bridge.

**Language & formatting**
- TypeScript strict mode enabled (`tsconfig.json`). Target ES2022, ESNext modules, `react-jsx`.
- No ESLint, Prettier, or EditorConfig is configured. Match surrounding style; do not introduce new formatters without discussion.
- Tailwind CSS v4 via `@tailwindcss/vite` plugin (utility classes only; no separate CSS files for components).
- Path alias `@/` ⇒ `./src/` (both Vite and tsconfig).

**State management**
- Zustand stores, vanilla (no devtools middleware by default). `appStore` has `persist`; others are in-memory.
- `fumenStore` uses `tetris-fumen` `Field` instances and `field.copy()` to clone. **Never `structuredClone()` a `Field`** — private fields get corrupted. This is a documented footgun.
- Field dimensions: 10 cols × 23 rows (y = 0..22); garbage row at y = −1. Display labels are 1-indexed; `field.at(x, y)` is 0-indexed.

**Async / process handling**
- All Rust commands are `async fn`. Stream stdout/stderr over Tokio channels; forward to frontend via `app.emit()`.
- **Never hold a `MutexGuard` across an `.await`** — extract the value first.
- `SfinderOutput` is returned as aResult-like struct; frontend maps `CommandStatus` (`idle | running | success | error | cancelled`) to UI state.

**Error handling**
- Rust: `Result<T, String>` everywhere; propagate with `?` or map to user-facing messages.
- Frontend: `try/catch` around `invoke()` with `commandStore.setError()`. Input validation in form components via controlled state.

**HTML rendering**
- sfinder output is raw HTML. Always run through `DOMPurify` (`sanitize-html.ts`) before `dangerouslySetInnerHTML`.

**i18n**
- English + 中文 dictionaries in `src/i18n/translations.ts`. Use `useTranslation()` (returning `useT()`) — keys are dot-notation (`t('commands.percent.title')`).

**Theme**
- Toggle `html.light` / `html.dark` class on the root element in `App.tsx` `useEffect`, driven by `appStore.theme` (`'light' | 'dark' | 'system'`).

## Important Files

| File | Why it matters |
|------|----------------|
| `src/main.tsx` | Frontend entry point |
| `src/App.tsx` | Route table + startup detection + theme |
| `src/routes/PercentPage.tsx`, `src/routes/PathPage.tsx` | Only fully-wired commands |
| `src/stores/fumenStore.ts` | Shared field-editor state — read before modifying grid logic |
| `src/stores/appStore.ts` | Settings + persistence; Java/JAR discovery flows through here |
| `src/lib/output-parser.ts` | Parsing logic mirrors Rust post-processing — keep in sync with `sfinder.rs` |
| `src-tauri/src/sfinder.rs` | JVM spawn, arg building, CSV post-processing (source of truth for CLI flags) |
| `src-tauri/src/commands.rs` | All 13 IPC handlers |
| `src-tauri/src/recognition.rs` | Screen-capture OCR (changes here affect capture accuracy) |
| `src-tauri/src/color_split.rs` | Piece auto-split (flood-fill + shape match + topo sort) |
| `src-tauri/tauri.conf.json` | Window size, CSP, bundle targets, shell-scope |
| `package.json` | Scripts, deps |
| `vite.config.ts` | React + Tailwind plugins, `@/` alias, port 1420 |
| `tsconfig.json` | Strict, ES2022, bundler resolution |

## Runtime / Tooling Preferences

- **Runtime**: Node.js 18+ (frontend) and Rust 1.94+ (backend). Java 17+ JDK required at runtime (on PATH or custom path in Settings). Bun and Deno are not used in this project.
- **Package manager**: **pnpm** is canonical (`pnpm-lock.yaml`, lockfileVersion 9.0 / pnpm v9). CI uses `npm ci` for bootstrapping. Either works locally; prefer pnpm to stay lockfile-consistent.
- **Build tool**: Vite 8 with `@vitejs/plugin-react` and `@tailwindcss/vite`.
- **Tauri CLI**: `@tauri-apps/cli ^2`. Run via `npm run tauri dev` / `npm run tauri build`.
- **Java distribution**: Either user-supplied JAR (path in Settings) or bundled `src-tauri/binaries/sfinder.jar`. A bundled JRE is optional (`scripts/bundle-jre.js` via `jlink`; ~35–40 MB).
- **No Docker, no ESLint, no Prettier** — adding either requires a deliberate project decision.

## Testing & QA

- **TypeScript/Vitest**: `vitest` v3.2.0 is declared and wired into `npm run test` / `npm run test:watch`, but **zero TS test files currently exist**. Use Vitest when adding frontend tests; follow the repo's non-tested baseline (no snapshot or coverage enforcement).
- **Rust/cargo**: Real tests live in `src-tauri/tests/` (`recognition_test.rs`, `board_tests.rs`) plus inline `#[cfg(test)]` modules in `recognition.rs` and `color_split.rs`. Run all with `cargo test`; the CI recognition suite uses `--test-threads=1` due to shared screenshot fixtures.
- **No coverage tooling** is configured on either side. Recognition tests compare OCR output against real tetr.io PNG fixtures in `src-tauri/tests/fixtures/`.
- **Manual QA**: After any sfinder-arg or parser change, run `npm run tauri dev`, draw a known field, and exercise Percent + Path end-to-end — the parsers must stay aligned with `sfinder.rs` post-processing.
