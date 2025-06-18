# Dev

use `cargo tauri dev` to start the development server. This will run the Tauri backend and the Vite frontend in development mode.

## iOS Development

use `cargo tauri ios dev`

### Set Development Team ID
set the `APPLE_DEVELOPMENT_TEAM` environment variable to your Team ID:
```bash
# example export APPLE_DEVELOPMENT_TEAM="AAAAAAA"
export APPLE_DEVELOPMENT_TEAM=<your_team_id>
```

### List available devices and simulators
```bash
# List all devices (real devices and simulators)
xcrun devicectl list devices

# List only simulators
xcrun simctl list devices

# Clean up unavailable simulators
xcrun simctl delete unavailable
```

### Development commands
```bash
# Auto-select available simulator
cargo tauri ios dev

# Use specific device by name
cargo tauri ios dev --device "iPhone 15"

# Use specific device by UDID (get UDID from list command above)
cargo tauri ios dev --device <device_udid>

# Debug

Use the Ctrl + Shift + i shortcut on Linux and Windows, and Command + Option + i on macOS to open the inspector.

# Tauri + Vue 3 + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Type Support For `.vue` Imports in TS

Since TypeScript cannot handle type information for `.vue` imports, they are shimmed to be a generic Vue component type by default. In most cases this is fine if you don't really care about component prop types outside of templates. However, if you wish to get actual prop types in `.vue` imports (for example to get props validation when using manual `h(...)` calls), you can enable Volar's Take Over mode by following these steps:

1. Run `Extensions: Show Built-in Extensions` from VS Code's command palette, look for `TypeScript and JavaScript Language Features`, then right click and select `Disable (Workspace)`. By default, Take Over mode will enable itself if the default TypeScript extension is disabled.
2. Reload the VS Code window by running `Developer: Reload Window` from the command palette.

You can learn more about Take Over mode [here](https://github.com/johnsoncodehk/volar/discussions/471).
