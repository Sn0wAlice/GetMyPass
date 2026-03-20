# GetMyPass

A simple, fast TUI password manager built in Rust with [Ratatui](https://ratatui.rs).

All your passwords and encrypted notes are stored in a single AES-256-GCM encrypted file at `~/.gmp/vault.enc`, protected by a master password derived with Argon2id.

Works on **Linux** and **macOS**.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)

## Features

- 🔑 **Password entries** — name, username, password, URL, notes
- 📝 **Encrypted notes** — standalone secure note entries
- 🔍 **Real-time search** — filter across all fields instantly
- 🎲 **Password generator** — configurable length, charset (upper, lower, digits, symbols)
- 📋 **Clipboard support** — copy passwords and usernames with a single key
- 🔒 **AES-256-GCM + Argon2id** — strong encryption with authenticated key derivation
- 👁️ **Password masking** — hidden by default, reveal on demand
- 💾 **Auto-save** — vault is saved on every change and on quit

## Install

### Homebrew (macOS & Linux)

```bash
brew tap Sn0wAlice/GetMyPass https://github.com/Sn0wAlice/GetMyPass
brew install gmpass
```

### Debian / Ubuntu (.deb)

Download the latest `.deb` from [Releases](https://github.com/Sn0wAlice/GetMyPass/releases):

```bash
# amd64
wget https://github.com/Sn0wAlice/GetMyPass/releases/latest/download/gmpass_0.1.0_amd64.deb
sudo dpkg -i gmpass_0.1.0_amd64.deb

# arm64
wget https://github.com/Sn0wAlice/GetMyPass/releases/latest/download/gmpass_0.1.0_arm64.deb
sudo dpkg -i gmpass_0.1.0_arm64.deb
```

### Download binary

Grab the tarball for your platform from [Releases](https://github.com/Sn0wAlice/GetMyPass/releases), extract, and move to your PATH:

```bash
tar xzf gmpass-darwin-arm64.tar.gz
sudo mv gmpass /usr/local/bin/
```

### Build from source

```bash
git clone https://github.com/Sn0wAlice/GetMyPass.git
cd GetMyPass
cargo build --release
sudo cp target/release/gmpass /usr/local/bin/
```

## Usage

```bash
gmpass
```

On first launch, you'll be asked to create a master password. This password encrypts your vault — **don't lose it**, there is no recovery.

### Keyboard shortcuts

#### Main list

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | View entry |
| `/` or `s` | Search |
| `n` | New password entry |
| `N` | New note entry |
| `e` | Edit selected entry |
| `d` | Delete selected entry |
| `c` | Copy password to clipboard |
| `u` | Copy username to clipboard |
| `q` | Quit (auto-saves) |

#### View entry

| Key | Action |
|-----|--------|
| `p` | Show / hide password |
| `c` | Copy password |
| `u` | Copy username |
| `e` | Edit |
| `Esc` | Back to list |

#### Edit entry

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | New line (notes) / Next field |
| `Ctrl+S` | Save |
| `Ctrl+G` | Generate password |
| `Esc` | Cancel |

#### Password generator

| Key | Action |
|-----|--------|
| `←` / `→` | Adjust length |
| `1` | Toggle uppercase |
| `2` | Toggle lowercase |
| `3` | Toggle digits |
| `4` | Toggle symbols |
| `r` | Regenerate |
| `Enter` | Accept |
| `Esc` | Cancel |

## Security

- **Encryption**: AES-256-GCM (authenticated encryption)
- **Key derivation**: Argon2id (memory-hard, GPU-resistant)
- **Storage**: single file `~/.gmp/vault.enc` — `salt || nonce || ciphertext`
- **Fresh salt & nonce** on every save
- **Atomic writes** via temp file + rename (no corruption on crash)
- **Memory zeroization** of sensitive data with `zeroize`

## License

MIT
