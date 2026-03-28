# launched-cli

A TUI tool for viewing and managing macOS `launchd` services.

## Features

- Browse all LaunchAgents and LaunchDaemons with status at a glance
- Tab-based filtering: **User** / **System** / **All** / **Timeline**
- Auto-grouped by label category (e.g. `com.example.backup.*` → `example`)
- Next run time calculation from `StartCalendarInterval` and `StartInterval`
- Error detail panel showing log tail for failed services
- Load/Unload services directly from the TUI
- Auto-refresh every 2 seconds

## Screenshot

```
┌launched───────────────────────────────────────────────────────┐
│ User │ System │ All │ Timeline                                │
└───────────────────────────────────────────────────────────────┘
┌───────────────────────────────────────────────────────────────┐
│      Label                                    Exit  Next Run  │
│                                                               │
│      ── backup ──                                             │
│> ✅  com.example.backup.daily                 0     07:00     │
│  ✅  com.example.backup.weekly                0     04/01 03:00│
│      ── myapp ──                                              │
│  ✅  com.myapp.web-server                     0     -         │
│  ❌  com.myapp.worker                         1     14:30     │
│  🔄  com.myapp.scheduler                     -     15:00     │
│      ── system ──                                             │
│  ⏸️  com.system.updater                       -     -         │
│                                                               │
└──────────────────── ✅ Success  ❌ Failed  🔄 Running  ⏸️  Unloaded ┘
```

### Error Detail Panel

When a failed service is focused, the error log is shown below the list:

```
┌ [Error] com.myapp.worker (exit: 1) ──────────────────────────┐
│ Error: connection refused to localhost:5432                    │
│   at Worker.connect (src/worker.ts:42)                        │
│   at processTicksAndRejections (node:internal/process/...)    │
└───────────────────────────────────────────────────────────────┘
```

### Timeline Tab

Shows upcoming scheduled runs sorted by time, without status columns:

```
┌launched───────────────────────────────────────────────────────┐
│ User │ System │ All │ Timeline                                │
└───────────────────────────────────────────────────────────────┘
┌───────────────────────────────────────────────────────────────┐
│    Label                                          Next Run    │
│                                                               │
│ >  com.example.backup.daily                       07:00       │
│    com.myapp.worker                               14:30       │
│    com.myapp.scheduler                            15:00       │
│    com.example.backup.weekly                      04/01 03:00 │
│    com.myapp.web-server                           -           │
│    com.system.updater                             -           │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

## Install

```bash
cargo install --path .
```

## Usage

```bash
launched-cli
```

## Key Bindings

| Key | Action |
|-----|--------|
| `←` `→` `h` `l` `Tab` | Switch tab |
| `↑` `↓` `j` `k` | Move cursor |
| `Enter` | Open action menu (Load/Unload) |
| `Ctrl+C` `q` | Quit |

## Scanned Directories

| Directory | Tab |
|-----------|-----|
| `~/Library/LaunchAgents/` | User |
| `/Library/LaunchAgents/` | System |
| `/Library/LaunchDaemons/` | System |

## Tech Stack

- [Rust](https://www.rust-lang.org/)
- [ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)
- [plist](https://crates.io/crates/plist)
- [chrono](https://crates.io/crates/chrono)
