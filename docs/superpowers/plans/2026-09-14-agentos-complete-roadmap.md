# AgentOS Completion Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Each task must keep its own red/green verification and evidence checkpoint.

**Goal:** Implement and verify every AgentOS capability that can be completed within the local Windows-first architecture, while keeping externally blocked gates explicit instead of claiming readiness without evidence.

**Architecture:** Preserve the vibe-kanban fork as the local Rust/SQLite/ACP control plane. UI remains a client of the existing providers and APIs; structured events and persisted execution records remain authoritative. New catalog, reconciliation, context, metrics, routing and orchestration contracts must be introduced incrementally and must not make a process, PTY, terminal text, or `exit 0` equivalent to semantic success.

**Tech Stack:** Rust server/executors/SQLite, React/TypeScript local web, ACP adapters, MCP stdio, Node-native pure contracts, Tailscale for private access, Tauri only after web validation.

**Spec:** `C:/Users/iagui/AI Projects/AgentOS-Prueba/docs/agentos/01-OBJETIVO.md`, `02-ARQUITECTURA.md`, `03-DECISIONES.md`, `04-ROADMAP.md`, `05-ESTADO-Y-GATES.md`, `06-ESPECIFICACION-UX.md`.

## Global Constraints

- Runtime that launches agents runs on native Windows; WSL/Docker are not substitutes.
- Identity remains `Project -> Workspace -> Worktree -> Session -> Execution`.
- Structured events and persisted control-plane state are authoritative; process liveness and terminal text are not.
- Human approval is required for credentials, installations, permissions, deploys, destructive actions and writes to user configuration.
- Remote access is authenticated and tailnet-only; never expose the local server to the open Internet.
- Engram context is namespaced, scoped, expiring and provenance-bearing; no global context dump.
- Measure duration, tokens/cost when reliable, taxonomy and semantic result before routing or swarm.
- Never write secrets to docs, logs, commits, fixtures, screenshots or evidence.
- Every implementation task uses TDD: failing contract first, expected red, minimal green, fresh verification, evidence and a small commit.
- Do not manually edit generated `routeTree.gen.ts` or shared generated Rust/TypeScript types.
- Do not install dependencies or run high-consumption builds without explicit authorization; classify unavailable toolchains as `BLOQUEADO`.

---

### Task 1: Reproducible frontend contract runner (L2)

**Files:**
- Create: `scripts/run-frontend-contracts.mjs`
- Create: `packages/ui/src/components/activeStateSemantics.test.ts` (already present on the L1 branch; include it in the inventory)
- Modify: `packages/web-core/src/shared/lib/diffDataAdapter.test.ts`
- Modify: root `package.json`
- Create: `docs/agentos/evidence/2026-09-14-agentos-frontend-contract-runner.md`

**Interfaces:**
- The runner receives no positional arguments, discovers only the explicit Node-native contract inventory, executes each test in a child process with `node --experimental-strip-types`, prints one result per file, and exits non-zero if any contract fails.
- `diffDataAdapter.test.ts` must become a Node-native contract or move into an explicit Vitest inventory; it must not be silently skipped.

- [ ] Write a failing runner contract that expects the explicit inventory and reports the Vitest-only file as unsupported rather than swallowing it.
- [ ] Run `node scripts/run-frontend-contracts.mjs`; record the expected red caused by the current `vitest` import.
- [ ] Migrate the diff adapter assertions to the chosen explicit runner without changing production behavior.
- [ ] Run every contract individually and through the runner; require per-file output and a reliable aggregate exit code.
- [ ] Add `pnpm run test:frontend-contracts` without adding a dependency.
- [ ] Run `git diff --check`, the i18n checker and the existing frontend guards; record missing tools separately.
- [ ] Commit as `test: make frontend contracts reproducible` only after the runner and evidence are green.

### Task 2: Inspector deep links with validated state (L3)

**Files:**
- Inspect/modify: `packages/web-core/src/project-routes/project-search.ts` and the workspace route files that own panel state.
- Inspect/modify: `packages/ui/src/components/Navbar.tsx` and workspace panel consumers.
- Modify: the pure state contract files under `packages/web-core/src/pages/workspaces/`.
- Create: a pure URL parsing/serialization contract beside the existing workspace state contracts.
- Create: `docs/agentos/evidence/2026-09-14-agentos-ui-deep-links.md`.

**Interfaces:**
- Valid panel IDs are exactly `changes`, `logs`, `preview`, and `git`; mobile tab IDs remain the existing `MobileTabId` union.
- Invalid URL values fall back to the current safe default without throwing, mutating providers, or editing generated route files.

- [ ] Write failing pure contracts for valid, invalid, absent and repeated panel/tab query parameters.
- [ ] Run the contracts red against the current in-memory-only behavior.
- [ ] Implement validated search-param parsing and serialization at the existing route/navigation boundary.
- [ ] Keep `useUiPreferencesStore` as derived/ephemeral state only; preserve WebSocket mounts and callbacks.
- [ ] Run the runner, TypeScript/checks available, `git diff --check`, and a read-only route smoke when a frontend runtime is already available.
- [ ] Commit as `feat: restore workspace inspector state from URL`.

### Task 3: Agent catalog and readiness contracts (P0)

**Files:**
- Create: a backend domain module under `crates/server/src/` for `AgentDescriptor`, readiness state and verified capability claims.
- Create: Rust unit tests for state transitions and evidence requirements.
- Modify: existing executor/adaptor registration only where the common catalog can consume it without changing launch behavior.
- Create: shared API types through the Rust source of truth, then regenerate using the repository command; never edit generated files directly.
- Create: `docs/agentos/evidence/2026-09-14-agentos-agent-catalog.md`.

**Interfaces:**
- Readiness progression is `unknown -> detected -> authenticated -> model_verified -> ready`, with `blocked` and `needs_action` carrying an explicit reason and next action.
- Declared capabilities and verified capabilities are separate records with provenance, timestamp and evidence reference.

- [ ] Write failing pure Rust tests for an executable without auth, auth without model verification, a verified model, provider error, and stale evidence.
- [ ] Run the tests red before adding the domain implementation.
- [ ] Implement the smallest domain contract without invoking providers or installing tools.
- [ ] Add read-only catalog exposure to the existing control-plane API only after the domain tests pass.
- [ ] Verify generated types from their Rust source and record any unavailable Rust toolchain as blocked.
- [ ] Commit as `feat: add governed agent readiness contract`.

### Task 4: Read-only session reconciliation and context grants (P0/P1)

**Files:**
- Create: provider-neutral reconciliation module under `crates/server/src/` or the existing services boundary after inspection.
- Create: pure tests/fixtures for absent, corrupt, incompatible, stopped, crashed and reconnecting provider stores.
- Create: domain types for `ContextSnapshot` and `ContextGrant` with scope, purpose, expiry, recipient and provenance.
- Create: `docs/agentos/evidence/2026-09-14-agentos-reconciliation-context.md`.

**Interfaces:**
- Reconciliation is read-only and best-effort; it emits `unknown`, `reconnecting` or `needs_attention` when authority is insufficient.
- A context grant cannot contain hidden reasoning, credentials, permission prompts or an unbounded conversation dump.

- [ ] Write failing tests for all fixture classes and secret/reasoning exclusion.
- [ ] Run them red.
- [ ] Implement adapters that normalize to canonical project/workspace/worktree/session/execution identity without provider-store writes.
- [ ] Add grant validation and snapshot truncation/provenance handling.
- [ ] Verify parser tests, secret-safe fixture scans and no mutation of provider stores.
- [ ] Commit as `feat: add read-only session reconciliation contracts`.

### Task 5: Guided launch preview and human confirmation (P1)

**Files:**
- Inspect/modify existing workspace creation and session-launch API/UI boundaries.
- Create a command/configuration preview contract and tests.
- Modify only the existing confirmation path; do not add direct frontend execution.
- Create: `docs/agentos/evidence/2026-09-14-agentos-guided-launch.md`.

**Interfaces:**
- Preview includes agent, model, project, workspace, worktree, non-secret variables, permissions and tools; secrets render as `[redacted]`.
- Confirmation is correlated to the execution authorization and cannot silently change the command being authorized.

- [ ] Write failing contracts for redaction, stable correlation, changed-preview rejection and destructive-action confirmation.
- [ ] Run red.
- [ ] Implement preview generation at the control-plane boundary and render it read-only in the existing create flow.
- [ ] Preserve all existing providers, callbacks, navigation and mutation authority.
- [ ] Verify pure contracts and a no-mutation UI smoke if a runtime is available.
- [ ] Commit as `feat: add human-reviewed launch preview`.

### Task 6: Runtime and human-blocking gates (G1/G2)

**Files:**
- Inspect/modify only the existing executor/event persistence paths.
- Integrate the isolated Antigravity semantic-result fix from `0cc0388e` only after source review and native build evidence.
- Create read-only smoke scripts/evidence under `docs/agentos/evidence/`.

**Interfaces:**
- A successful gate requires exact SHA, command, input, output/events, exit code, semantic result, final status, worktree isolation and clean status.
- `blocked` must come from a structured question/permission event and support response, cancel, timeout, duplicate and restart cases.

- [ ] Write or extend fixtures for ACP error plus `completed/0`, closed output without result, structured question, response and cancellation.
- [ ] Run fixtures red against any false-success path.
- [ ] Implement only the minimal semantic-result/event fix.
- [ ] Run native Windows checks and the real read-only provider tasks only when authentication/quota and required toolchain are already available.
- [ ] Classify G1/G2 as `VERIFICADO`, `PROBABLE`, `BLOQUEADO` or `DESCONOCIDO`; do not infer readiness from process existence.
- [ ] Commit each executor fix separately and update the canonical evidence.

### Task 7: Mobile local-first, metrics and Engram gates (G3–G5)

**Files:**
- Modify only existing responsive/control surfaces for structured state and human control.
- Add execution metric persistence/query contracts in the existing Rust/SQLite boundaries.
- Define Engram MCP stdio namespace/grant contract and failure-tolerant integration tests before any provider wiring.
- Create separate evidence files for mobile, metrics and Engram.

**Interfaces:**
- Mobile uses the same authenticated control plane and authority as desktop; no parallel state or open Internet bind.
- Metrics distinguish unavailable from zero and record semantic result separately from exit code.
- Engram failure cannot prevent execution-state persistence.

- [ ] Write failing pure contracts for mobile stale/reconnecting state, metric availability, namespace isolation and Engram outage.
- [ ] Run red.
- [ ] Implement contracts and storage/query behavior in small commits.
- [ ] Verify with local read-only browser smoke first; do not configure Tailscale or credentials automatically.
- [ ] When the user authorizes external setup, verify MagicDNS, authentication and phone control with evidence.

### Task 8: Explainable routing, orchestrator/swarm and Tauri (G6/Fase 6–8)

**Files:**
- Create rule-based routing domain contracts and decision explanations.
- Extend existing MCP/session tools only through the control-plane authority.
- Create orchestration dependency/result contracts and tests before parallel execution.
- Modify Tauri packaging only after web/runtime gates are green and native Windows toolchain is available.
- Create separate evidence for routing, swarm and Tauri.

**Interfaces:**
- Routing rules are explicit, reversible and human-reviewable; no opaque learned model is introduced first.
- Orchestration preserves project/workspace/worktree identity, dependencies, approvals, correlation IDs and consolidated results.
- Tauri is a packaging surface, not a second source of session truth.

- [ ] Write failing contracts for routing explanations, fallback, dependency failure, partial swarm result and human review.
- [ ] Run red.
- [ ] Implement the smallest rule engine and MCP orchestration primitives.
- [ ] Verify with synthetic fixtures before any live multi-agent run.
- [ ] Run Tauri only with authorized native toolchain and record the exact installer/build evidence.

### Task 9: Original AgentOS identity and final consolidation

**Files:**
- Create an approved master SVG and derived assets only after explicit human approval of one of three concepts.
- Modify AppBar/onboarding/remote/metadata/Tauri asset references in a dedicated branding commit.
- Update `docs/agentos/05-ESTADO-Y-GATES.md` only from current evidence.

**Interfaces:**
- The placeholder `A` is not evidence of final identity; all variants derive reproducibly from the approved source.
- Branding changes do not alter runtime authority, providers, sessions, routes or state contracts.

- [ ] Produce three concepts and request human selection; no asset integration before selection.
- [ ] Write visual/accessibility acceptance checks for 16–512 px, dark/light/high-contrast backgrounds and accessible name preservation.
- [ ] Implement the approved source and derived assets.
- [ ] Verify desktop/mobile/Tauri surfaces at the final SHA.
- [ ] Reconcile all evidence and classify every roadmap phase and G1–G6 honestly.

## Stop Conditions and Explicit Blockers

- Stop before installations, credential changes, Tailscale setup, Docker, deploys, pushes, merges or destructive actions unless separately authorized.
- Stop before claiming a runtime gate when provider authentication, quota, `libclang.dll`, Tauri toolchain, phone access or Engram service evidence is missing.
- If a test runner or compiler is absent, add a reproducible fallback only when it does not change gate meaning; otherwise report `BLOQUEADO` with the exact command and exit code.

## Completion Definition

The roadmap is complete only when every task above has an implementation commit,
fresh verification evidence, current SHA, command, exit code, input, artifact,
and explicit residual limits. Until then, the canonical state must continue to
describe the project as partial and must not mark G1–G6 or the full migration
complete.
