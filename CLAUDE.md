This is a Roblox bootstrapper that runs the macOS client on Linux. The UI toolkit used is GTK4 with libadwaita.

There are some things to know before making changes:

- When making a new component, add it in `src/components/**` and export them.
- When making a new page, give it its own folder `src/pages/<name>/` and export it in `src/pages.rs`. The page itself goes in `mod.rs`, and functions that only that page uses go in separate files inside the same folder.
- Utility functions go to `src/utils.rs`.
- When making changes to the UI that need new state properties, you may modify in `src/state.rs` and use the abstractions in your changes, otherwise when state is needed but only existing ones, then still use the abstractions.
