# Localhost Linker

A tray app that finds the dev servers running on your machine and gives you a QR code your
phone can scan to open them over Wi-Fi. No more typing `192.168.1.5:3000` on a phone keyboard.

Click the tray icon to open the popup. Servers that a phone can reach show a QR code and a
**LAN** pill. Servers bound only to `localhost` (Vite's default) show a **localhost only** pill
and the flag to fix it, such as `npm run dev -- --host`.

## How it works

- Every 2 seconds it reads the OS table of listening TCP sockets (no admin needed) and the
  owning process. Only dev runtimes (node, python, dotnet, ...) or well-known dev ports are
  shown; tick **Show all** for everything else.
- A server is reachable from the LAN if it listens on `0.0.0.0`, `::`, or the LAN IP itself.
- The LAN IP skips VPN, WSL, Hyper-V and other virtual adapters and prefers `192.168.x.x`,
  then `10.x.x.x`, then `172.16-31.x.x`, using the OS default route only to break ties.
  This matters because a VPN like NordLynx usually owns the default route.
- The popup only refreshes when the set of servers or the IP actually changes.

Tray menu: Show servers, Launch at login, Quit. In the popup: click a row to select it,
double-click or press Ctrl+C to copy its URL, Esc to hide.

## Develop

Requires Rust (stable, MSVC), the Tauri CLI, and the Visual Studio C++ build tools.

```sh
cargo install tauri-cli --version "^2.0.0" --locked

cd src-tauri
cargo test                                  # unit tests
cargo test live_scan -- --ignored --nocapture  # print what the app sees on this machine
cargo tauri dev                             # run it
cargo tauri build                           # NSIS installer in target/release/bundle/nsis
```

## Layout

| Path | Purpose |
|---|---|
| `src-tauri/src/lib.rs` | Tray, popup window, polling thread, commands |
| `src-tauri/src/scanner.rs` | Socket table to server list: filtering, dedupe, reachability |
| `src-tauri/src/ip.rs` | LAN IPv4 selection |
| `src-tauri/src/qr.rs` | QR code as inline SVG |
| `ui/index.html` | The popup, plain HTML with no build step |

## Known limits

- Windows is the only platform built and tested so far. The crates are cross-platform.
- If a phone still cannot connect to a LAN server, Windows Firewall is usually blocking
  the dev server's process. The app cannot change that for you.
- Servers inside WSL 2 in its default NAT mode are not reachable from a phone. Enable
  mirrored networking in WSL to fix that.
