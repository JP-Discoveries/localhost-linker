# The 'Localhost' Linker

A tiny system tray app that scans your local machine for active development servers (port 3000, 8080, etc.) and instantly generates a QR code or local IP link for your phone to test mobile views. It is for solo developers who constantly get frustrated typing '192.168.1.5:3000' into their mobile browser.

## Status

Windows MVP is built and tested with Tauri v2. See `README.md` for how it works, how to run
it, and known limits. `BRIEF.md` is the original research brief; the build departs from it on
purpose in a few places:

- Ports are found by reading the OS socket table (`listeners` crate), not by trying to bind them.
- The QR code lives in a popup window; tray tooltips cannot show images.
- The LAN IP ranks physical adapters over the OS default route, because VPNs such as NordLynx
  own the default route on this machine.
- Each server is flagged as LAN-reachable or localhost-only, since Vite binds localhost by default.

Verify changes with `cargo test` and `cargo test live_scan -- --ignored --nocapture` in
`src-tauri`, then `cargo tauri dev`.
