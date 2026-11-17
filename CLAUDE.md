Read [AI_USAGE_POLICY.md](AI_USAGE_POLICY.md) before making changes.

This is a Roblox bootstrapper that runs the macOS client on Linux. The UI toolkit used is iced.

There are some things to know before making changes:

- MOST IMPORTANT: Make the code as self explanatory as possible, no comments.
- The UI follows iced's model: views are functions of `AppState` and page/component state that return `Element<Message>`, and changes happen in `update`. Views must stay cheap and side-effect free: never touch the filesystem or network from a view, cache such results in the page's state and refresh them from `update`.
- `quadra::act` turns a change to `AppState` (or a backend call) into a `Message`. Reactions to state changes that need the window, such as opening a dialog, go through the reactions queue in `src/quadra.rs`.
- Keep the widget tree shape stable around stateful widgets like text inputs: reserve space for optional elements instead of inserting them, or the widget loses its focus.
- The glib main loop is replaced by `src/main_loop.rs`, which runs futures and timers on the UI thread and is woken by iced. Use it instead of spawning work on the UI thread any other way.
- When making a new component, add it in `src/components/**` and export them. Shared look and widget styles live in `src/components/style.rs`, rows and groups in `src/components/preferences.rs`.
- When making a new page, give it its own folder `src/pages/<name>/` and export it in `src/pages.rs`. The page itself goes in `mod.rs`, and functions that only that page uses go in separate files inside the same folder.
- Utility functions go to `src/utils.rs`.
- Backend code (installing, downloading, running Rosemary and Roblox) goes in `src/backend/`, with its public actions in `src/backend.rs`. It talks to the UI only through `AppState`; long work runs in the background with `run_task` (user-started work: marks Quadra busy, shows progress, failures open an error dialog) or `run_quietly` (fetches the user shouldn't notice: nothing is blocked, the caller handles the `Result`, usually by logging it).
- When making changes to the UI that need new state properties, you may modify in `src/state.rs` and use the abstractions in your changes, otherwise when state is needed but only existing ones, then still use the abstractions.
