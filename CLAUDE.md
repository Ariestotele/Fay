# Fay — Claude working agreement

Fay is a personal **desktop launcher hub** for Windows: a minimal, dark, "techy"
landing page that opens apps and snaps groups of windows into position across
two monitors.

## Read these FIRST (in order) every new session

1. `docs/STATE.md`   — what's done, what's next. The single source of truth for progress.
2. `docs/ARCHITECTURE.md` — how Fay is built and why it's shaped this way.
3. `docs/DECISIONS.md` — locked decisions. Do **not** re-debate these.

Reading those three small files = full context. Don't re-explore the repo from
scratch; that wastes the user's usage budget.

## Golden rules (this project optimizes for low token usage)

- **Routine changes are JSON edits, not AI work.** Adding an app/scene = editing
  `src/apps.config.json`. Don't spin up a session for that.
- **End every session by updating `docs/STATE.md`.** This is the hand-off contract.
- **`docs/DECISIONS.md` is append-only.** New decisions get appended with a date;
  old ones are never silently rewritten.
- **Keep the frontend config-driven.** Never hardcode an app/scene in the UI code.
- **Stay in scope.** Fay is a launcher, not a window manager. PowerToys does the
  positioning (see ARCHITECTURE). Don't build a positioning engine.

## How to run

See `docs/SETUP.md`. Short version once toolchain is installed:

```
npm install
npm run dev      # Tauri dev window
```

## Branch

Develop on `claude/charming-ramanujan-shirak`. Commit with clear messages. Push
to that branch. Never open a PR unless explicitly asked.
