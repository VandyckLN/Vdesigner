# Third-party licenses

Vdesigner is MIT-licensed (see `LICENSE`). It depends on the open-source
packages listed below. This list covers direct dependencies only, not the
full transitive dependency graph.

## Rust crates

### Workspace dependencies (`Cargo.toml`)

| Crate | License |
|---|---|
| image | MIT OR Apache-2.0 |
| serde | MIT OR Apache-2.0 |
| thiserror | MIT OR Apache-2.0 |
| webp | MIT OR Apache-2.0 (crate); bundles `libwebp` (C), BSD-3-Clause |
| ravif | BSD-3-Clause |
| rgb | MIT |
| fast_image_resize | MIT OR Apache-2.0 |
| imageproc | MIT |
| resvg | Apache-2.0 OR MIT |

### `src-tauri` dependencies (`src-tauri/Cargo.toml`)

| Crate | License |
|---|---|
| tauri | Apache-2.0 OR MIT |
| tauri-plugin-dialog | Apache-2.0 OR MIT |
| serde_json | MIT OR Apache-2.0 |
| base64 | MIT OR Apache-2.0 |

## JavaScript packages (`ui/package.json`)

### Dependencies

| Package | License |
|---|---|
| @tauri-apps/api | Apache-2.0 OR MIT |
| @tauri-apps/plugin-dialog | MIT OR Apache-2.0 |
| react | MIT |
| react-dom | MIT |

### Dev dependencies

| Package | License |
|---|---|
| @testing-library/jest-dom | MIT |
| @testing-library/react | MIT |
| @testing-library/user-event | MIT |
| @types/node | MIT |
| @types/react | MIT |
| @types/react-dom | MIT |
| @vitejs/plugin-react | MIT |
| jsdom | MIT |
| typescript | Apache-2.0 |
| vite | MIT |
| vitest | MIT |
