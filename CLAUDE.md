Read [AI_USAGE_POLICY.md](AI_USAGE_POLICY.md) before making changes.

This is a Roblox bootstrapper that runs the macOS client on Linux. The UI toolkit used is GTK4 with libadwaita.

There are some things to know before making changes:

- MOST IMPORTANT: Make the code as self explanatory as possible, no comments.
- When making a new component, add it in `src/components/**` and export them.
- When making a new page, give it its own folder `src/pages/<name>/` and export it in `src/pages.rs`. The page itself goes in `mod.rs`, and functions that only that page uses go in separate files inside the same folder.
- Utility functions go to `src/utils.rs`.
- Backend code (installing, downloading, running Rosemary and Roblox) goes in `src/backend/`, with its public actions in `src/backend.rs`. It talks to the UI only through `AppState`; long work runs in the background with `run_task` (user-started work: marks Quadra busy, shows progress, failures open an error dialog) or `run_quietly` (fetches the user shouldn't notice: nothing is blocked, the caller handles the `Result`, usually by logging it).
- When making changes to the UI that need new state properties, you may modify in `src/state.rs` and use the abstractions in your changes, otherwise when state is needed but only existing ones, then still use the abstractions.
