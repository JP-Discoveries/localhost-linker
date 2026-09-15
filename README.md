# Localhost Linker

A tray app for Windows, macOS and Linux that finds the dev servers running on your machine and gives you a QR code
your phone can scan to open them over Wi-Fi. No more typing `192.168.1.5:3000` on a phone keyboard.

<p align="center"><img src="docs/screenshot.png" width="336" alt="Localhost Linker popup showing a QR code for a dev server on port 3000"></p>

## Download

Get the file for your OS from the
[latest release](https://github.com/JP-Discoveries/localhost-linker/releases/latest).

| OS | File | Notes |
|---|---|---|
| Windows 10/11 | `win-x64-setup.exe` or `win-x64-portable.zip` | Needs the WebView2 runtime, which Windows 11 already has. |
| macOS 10.15+ | `macos-universal.dmg` | Works on Apple Silicon and Intel. Drag the app to Applications. |
| Linux | `linux-x86_64.AppImage`, `linux-amd64.deb` or `linux-x86_64.rpm` | Needs a desktop with tray support (see below). |

The builds are not code-signed yet, so each OS warns you the first time:

- **Windows:** SmartScreen says "Windows protected your PC". Click **More info**, then **Run anyway**.
- **macOS:** the first launch is blocked. Open **System Settings › Privacy & Security** and click
  **Open Anyway**, or run `xattr -dr com.apple.quarantine "/Applications/Localhost Linker.app"`.
- **Linux:** make the AppImage executable with `chmod +x`.

## Features

### Finds your servers automatically
- Reads the OS table of listening TCP ports every 2 seconds, with no admin rights needed.
- Shows the process that owns each port: node, python, dotnet and so on.
- Hides OS plumbing and background apps. Tick **Show all** to see everything.

### Tells you whether a phone can actually reach it
- **LAN** means the server listens on all interfaces, so the QR code works.
- **localhost only** means a phone can't connect. The popup shows the flag that fixes it,
  such as `npm run dev -- --host` for Vite.

### Picks the right IP address
- Skips VPN, WSL, Hyper-V and other virtual adapters.
- Prefers `192.168.x.x`, then `10.x.x.x`, then `172.16-31.x.x`, using the OS default route
  only to break ties. This matters because a VPN usually owns the default route.

### Stays out of the way
- Click the tray icon to open the popup; click away or press Esc to hide it. On Linux, choose
  **Show servers** from the tray menu instead.
- Copy a URL with the button, a double-click, or Ctrl+C (Cmd+C on macOS).
- Optional **Launch at login**, from the popup or the tray menu.
- Around 0.1% CPU when idle, and it only talks to the popup while it is open.

## Tech stack

Rust and [Tauri v2](https://v2.tauri.app/). The popup is a single HTML file with no build step.
Key crates: `listeners` for the socket table, `local-ip-address`, and `qrcode`.

## Building

Requires Rust stable and the Tauri CLI, plus:

- **Windows:** the Visual Studio C++ build tools (MSVC toolchain).
- **macOS:** Xcode Command Line Tools (`xcode-select --install`).
- **Linux (Debian/Ubuntu):**
  `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev`

```sh
cargo install tauri-cli --version "^2.0.0" --locked

cd src-tauri
cargo test                                     # unit tests
cargo test live_scan -- --ignored --nocapture  # print what the app sees on this machine
cargo tauri dev                                # run it
cargo tauri build                              # installers for this OS in target/release/bundle
```

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds all three platforms and
publishes them as one GitHub release.

## Project layout

| Path | Purpose |
|---|---|
| `src-tauri/src/lib.rs` | Tray, popup window, polling thread, commands |
| `src-tauri/src/scanner.rs` | Socket table to server list: filtering, dedupe, reachability |
| `src-tauri/src/ip.rs` | LAN IPv4 selection |
| `src-tauri/src/placement.rs` | Where the popup opens relative to the tray |
| `src-tauri/src/qr.rs` | QR code as inline SVG |
| `ui/index.html` | The popup |

## Known limits

- Windows is tested on real hardware, and Linux on Ubuntu 24.04 (GNOME) in a VM. The macOS build
  passes CI but has not yet been tried on a Mac, so please open an issue if something looks wrong.
- Linux tray icons don't report clicks, so the popup opens from the tray menu. Stock GNOME needs
  the AppIndicator extension to show tray icons at all; KDE, Cinnamon, XFCE and Ubuntu's GNOME
  work out of the box.
- On Linux, process names can be missing for servers started by another user.
- If a phone still can't connect to a **LAN** server, a firewall is usually blocking the dev
  server's process. The app can't change that for you.
- Servers inside WSL 2 in its default NAT mode aren't reachable from a phone. Turn on
  mirrored networking in WSL to fix that.

## License

[MIT](LICENSE)
