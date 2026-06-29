# rfont Desktop

Desktop application for rfont - a font subsetting tool built with Tauri 2.0, Vue 3, and TypeScript.

## Features

- 📁 **Load Font Files**: Support for TTF, WOFF, and WOFF2 formats
- 📊 **View Font Information**: Display font metadata including family and glyph count
- ✂️ **Create Font Subsets**: Generate optimized font files containing only specified characters
- 💾 **Export Subsetted Fonts**: Save subsetted fonts with custom output paths

## Prerequisites

- Node.js (v18 or later)
- pnpm (v8 or later)
- Rust (latest stable)

## Getting Started

### 1. Install Dependencies

```bash
pnpm install
```

### 2. Run Development Mode

```bash
pnpm tauri dev
```

This will:
- Start the Vite development server
- Compile the Rust backend
- Open the application window
- Enable hot-reload for frontend changes

### 3. Build for Production

```bash
pnpm tauri build
```

This creates platform-specific installers in `src-tauri/target/release/bundle/`.

## Project Structure

```
rfont-desktop/
├── src/                    # Vue 3 + TypeScript frontend
│   ├── App.vue            # Main application component
│   ├── main.ts            # Application entry point
│   └── vite-env.d.ts      # TypeScript declarations
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri commands implementation
│   │   └── main.rs        # Application entry point
│   ├── capabilities/      # Permission configurations
│   ├── icons/             # Application icons
│   ├── build.rs           # Build script
│   └── tauri.conf.json    # Tauri configuration
├── package.json           # Frontend dependencies
└── Cargo.toml             # Workspace configuration
```

## Technology Stack

- **Frontend**: Vue 3.5 + TypeScript 5.6 + Vite 6
- **Backend**: Rust + Tauri 2.x
- **Package Manager**: pnpm
- **Font Processing**: rfont core library


## Development

For detailed development guide, see [DEVELOPMENT.md](./DEVELOPMENT.md).

## License

MIT
