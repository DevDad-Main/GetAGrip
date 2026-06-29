# GetAGrip

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.86%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131.svg?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg?logo=svelte)](https://svelte.dev/)
[![Discord](https://img.shields.io/discord/123456789012345678?logo=discord&label=Chat)](https://discord.gg/getagrip)

> **GetAGrip** — a modern, cross‑platform database IDE built with a Rust core, Tauri shell, and Svelte frontend. Fast, intelligent, and runs entirely on your machine.

![GetAGrip Screenshot](https://github.com/DevDad-Main/GetAGrip/raw/main/assets/screenshot.png)

---

## ✨ Features

- **Multi‑Database Support** – Connect to PostgreSQL, MySQL/MariaDB, Microsoft SQL Server, and SQLite with a unified UI.
- **Smart SQL Completion** – Context‑aware autocompletion powered by:
  - Built‑in schema‑aware intelligence engine
  - Optional Language Server Protocol (LSP) integration for advanced completions (when LSP servers are available)
- **Rich Query Editor** – Monaco‑powered editor with syntax highlighting, diagnostics, and inline documentation.
- **Results & History** – View query results in a grid, export to CSV/JSON, and browse execution history.
- **Database Explorer** – Browse schemas, tables, views, and columns with drag‑&‑drop support.
- **Modern UI** – Dark theme (Darcula) with multiple color schemes, responsive layout, and smooth animations.
- **Secure Connections** – Credentials stored encrypted via the system keyring or a local vault.
- **Extensible** – Plugin system for custom drivers, dialects, and features.

## 🚀 Installation

### Prerequisites

- **Rust** ≥ 1.86 (install via [rustup](https://rustup.rs/))
- **Node.js** ≥ 20 (for frontend build)
- **Tauri CLI** (`cargo install tauri-cli@^2`)
- **System dependencies** (Linux only):
  ```bash
  # Debian/Ubuntu
  sudo apt-get install webkit2gtk-4.1 libappindicator3 patchelf

  # Fedora
  sudo dnf install webkit2gtk4.1 libappindicator-gtk3 patchelf

  # Arch
  sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 patchelf
  ```

### Build from Source

```bash
# 1️⃣ Clone the repository
git clone https://github.com/DevDad-Main/GetAGrip.git
cd GetAGrip

# 2️⃣ Install frontend dependencies
cd apps/desktop/frontend
npm install

# 3️⃣ Build & run (development)
cd ..
npm run tauri dev   # starts Vite dev server + Tauri window

# 4️⃣ Production build
npm run tauri build
# Binary appears in ./apps/desktop/src-tauri/target/release/
```

### Pre‑built Binaries

Download the latest release for your platform from the [GitHub Releases](https://github.com/DevDad-Main/GetAGrip/releases) page.

| Platform | Package |
|----------|---------|
| Windows  | `.exe` (MSI/portable) |
| macOS    | `.dmg` (AppleSilicon` / `.Intel` |
| Linux    | `.AppImage` (also available via AUR: `getagrip-bin`) |

---

## 📖 Documentation

Comprehensive guides are available on the project website: <https://getagrip.vercel.app>

### Quick Start

1. Launch GetAGrip.
2. Click the **+** button in the sidebar (or press `Ctrl+Shift+N`) to add a new connection.
3. Choose your database type, fill in host/port/credentials, and click **Test Connection**.
4. After a successful test, click **Connect** – the explorer will populate with your schemas.
5. Open a new query tab (`Ctrl+T`), write SQL, and hit **Ctrl+Enter** to run.

### Advanced: Enabling LSP‑Powered Completion

GetAGrip can optionally use Language Server Protocol (LSP) servers for richer, database‑specific completions (e.g., function signatures, advanced syntax). To enable:

1. **Install an LSP server** for your database:
   - **PostgreSQL**: `pglsp` (from [supabase-community/postgres-language-server](https://github.com/supabase-community/postgres-language-server))
   - **MySQL/MariaDB**: `mysql-language-server` (e.g., via `npm i -g @sqliteorg/mysql-language-server`)
   - **SQL Server**: `sql-language-server` (from [joe-re/sql-language-server](https://github.com/joe-re/sql-language-server))
   - **SQLite**: `sqlite-lsp` (e.g., via `pip install sql-language-server` or dedicated binary)

2. **Make the binary discoverable**:
   - Add its directory to your `PATH`, **or**
   - Set an environment variable before launching GetAGrip:
     ```bash
     # Example for PostgreSQL
     export POSTGRES_LSP_PATH=/usr/local/bin/pglsp
     ./getagrip   # or run from your IDE
     ```

   Supported variables: `POSTGRES_LSP_PATH`, `MYSQL_LSP_PATH`, `MSSQL_LSP_PATH`, `SQLITE_LSP_PATH`.

3. **Restart** the application if it was already running. You’ll see a log line like:
   ```
   INFO  getagrip_intelligence::lsp_client: Registered PostgreSQL LSP provider
   ```

> **Tip:** Even without an LSP server, GetAGrip’s built‑in intelligence engine provides schema‑aware completions based on the cached metadata.

### Configuration

- **Themes**: Change via `View → Theme` (Darcula, Catppuccin Mocha, Nord, One Dark, Solarized Light/Dark).
- **Keybindings**: View/Edit under `File → Keyboard Shortcuts`.
- **Settings**: Adjust query timeout, result row limit, enable/disable autosave, etc.

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:

- Reporting bugs
- Suggesting features
- Submitting pull requests
- Code style and testing guidelines

---

## 📄 License

Licensed under either of:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

At your option.

---

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) – for the secure, lightweight desktop runtime.
- [Svelte](https://svelte.dev/) – for the reactive frontend framework.
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) – the code editor powering VS Code.
- The open‑source LSP community (pg_lsp, mysql‑language‑server, sql‑language‑server, sqlite‑lsp) for enabling rich language support.

---

**Made with ❤️ by the GetAGrip team.**  
Visit us at <https://getagrip.vercel.app> for news, tutorials, and roadmap updates.