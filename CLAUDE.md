This is a Roblox bootstrapper that runs the macOS client on Linux. The UI toolkit used is GTK4 with libadwaita.

There are some things to know before making changes:

- When making a new component, add it in `src/components/**` and export them.
- When making a new page, add it in `src/*_page.rs` and export it.
- Utility functions go to `src/utils.rs`.
