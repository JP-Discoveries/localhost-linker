# The 'Localhost' Linker

> A tiny system tray app that scans your local machine for active development servers (port 3000, 8080, etc.) and instantly generates a QR code or local IP link for your phone to test mobile views. It is for solo developers who constantly get frustrated typing '192.168.1.5:3000' into their mobile browser.

It removes the friction of switching contexts between desktop development and mobile testing, solving a daily annoyance with zero setup.

*Idea #38 from Big Brain Ideas · proposed 2026-09-14 · judged 2026-09-14 · researched 2026-09-14*

---

## What already exists
None of the provided sources cover existing solutions, pricing, or specific libraries for this concept. Based on general knowledge of the development ecosystem, several tools already address parts of this problem:
*   **Browser Extensions:** Extensions like "Localhost Link" (Chrome/Edge) or "IP Address" allow generating links from within the browser, but they often require manual triggering rather than auto-scanning active ports.
*   **System Tray Utilities:** Tools like "Port Scanner" (Windows) or "Lantern" (macOS) can monitor ports, but they rarely integrate directly with mobile browsers or QR code generation in a seamless "one-click" workflow.
*   **CLI Tools:** Utilities like `ngrok` or `localtunnel` provide tunneling, but they introduce latency, require account creation, and are overkill for simple localhost testing.
*   **IDE Plugins:** VS Code extensions exist to open URLs, but they do not typically generate QR codes for mobile devices automatically.

## Where this idea can win
The primary gap is the **friction of context switching**. Existing tools force the developer to either:
1.  Open a browser extension, find the port, copy the IP, and manually type it into a mobile browser.
2.  Use a tunneling service that adds network latency and complexity.
3.  Manually configure a static IP address in their router (not feasible for dynamic home networks).

This tool wins by automating the entire loop: detection -> IP resolution -> QR generation -> display. The value proposition is "zero-setup" testing. If the app can detect a server starting on port 3000 and immediately show a scannable QR code in the tray without user intervention, it solves the specific annoyance of "I just started the server, how do I see it on my phone?"

## Who it is for
*   **Solo Frontend Developers:** Working on React, Vue, or Angular apps who frequently test responsive designs on their phones.
*   **Mobile-First Developers:** Those building React Native or Flutter apps who need to verify UI on a real device while developing on desktop.
*   **Freelancers/Consultants:** Who switch between multiple projects and need to quickly share a local build with a client on a different device without setting up a tunnel.
*   **Not for:** Teams requiring production-grade tunneling, or developers who always have a static local IP address they can hardcode.

## The MVP
*   **Port Scanning:** Monitor a predefined list of common development ports (3000, 8080, 5173, 4200, 3001) for active TCP connections.
*   **IP Resolution:** Automatically resolve the local IP address of the machine (handling IPv4/IPv6 and dynamic changes).
*   **Link Generation:** Construct the full URL (`http://<local-ip>:<port>`).
*   **QR Code Display:** Render a QR code for the generated URL in the system tray.
*   **Click Action:** Clicking the tray icon copies the URL to the clipboard and opens the QR code.
*   **Auto-Start:** Launch on system boot and monitor ports in real-time.

## How to build it
**Stack:**
*   **Language:** Rust or Go.
    *   *Why:* Both offer native access to OS networking APIs (listening for port opens) and efficient binary generation. Rust provides better memory safety and cross-platform consistency; Go is slightly faster to write for simple networking tasks. Given the need for low-level OS integration (system tray, network sockets), Rust is the superior choice for long-term maintainability and performance.
*   **System Tray:**
    *   *Windows:* `tray-icon` crate (Rust) or `tray` package.
    *   *macOS:* `menubar` crate (Rust) or `menubar` (Go).
    *   *Linux:* `tray` crate (Rust) or `libappindicator`.
*   **QR Code Generation:**
    *   *Rust:* `qrcode` crate.
    *   *Go:* `github.com/skip2/go-qrcode`.
*   **Network Monitoring:**
    *   *Rust:* `tokio` with `tokio::net::TcpListener` or `nucleo` for socket monitoring.
    *   *Go:* `net` package with `net.Listen` or `netpoll`.

**Architecture:**
A single binary that runs as a background service. It uses a loop to check socket availability on specific ports. When a socket is bound, it resolves the local hostname/IP, formats the string, generates a QR image buffer, and updates the tray icon. Clicking the icon triggers a `clipboard` write and `os.OpenURL`.

## Step-by-step plan
1.  **Setup Project & CI:** Initialize a Rust project. Set up GitHub Actions to build binaries for Windows, macOS, and Linux.
2.  **Implement Port Monitoring:** Create a function that attempts to bind to a specific port (e.g., 3000). If the bind fails with "Address already in use," the port is active. Implement a loop checking a list of common ports (3000, 8080, 5173, 4200, 3001).
3.  **Implement IP Resolution:** Write logic to get the local IP address. Handle edge cases where the machine has multiple interfaces (Wi-Fi vs Ethernet) and prioritize the active interface.
4.  **Build System Tray Integration:** Implement the tray icon for the target OS. Ensure it shows a tooltip with the current URL and a click handler to copy the URL to the clipboard.
5.  **Add QR Code Generation:** Integrate a QR code library. Create a function that takes the URL string and returns an image buffer. Display this image in the tray tooltip or as a secondary icon.
6.  **Add Auto-Start:** Implement the logic to register the app to launch on user login (using `launchd` on macOS, Task Scheduler on Windows, and systemd/user units on Linux).
7.  **Refine UX:** Add a "Stop Listening" toggle in the tray menu. Add logging to a temporary file for debugging. Ensure the app exits cleanly when the user logs out.
8.  **Testing:** Test on a machine with dynamic IP addresses. Verify that the QR code works on both iOS Safari and Android Chrome.

## Risks and open questions
*   **Dynamic IP Changes:** If the local IP changes (common on home networks with DHCP), the QR code will become invalid.
    *   *Mitigation:* The app must re-scan and update the QR code immediately upon IP change, or the user must refresh the tray.
*   **Port Conflicts:** What if a user runs two servers on the same port?
    *   *Mitigation:* The scanner should detect *any* active listener on the port, not just the first one. The UI might need to show "Port X is in use" without specifying which app, or list all active apps on that port.
*   **Firewall Restrictions:** Some corporate or strict home firewalls may block inbound connections even if the port is open locally.
    *   *Mitigation:* The app cannot fix this; it can only warn the user if the port is open but the connection fails (requires a test connection, which might be blocked by the OS).
*   **Cross-Platform Tray Consistency:** System tray behavior varies wildly between Windows, macOS, and Linux.
    *   *Mitigation:* Use a robust cross-platform crate (like `tray-icon` in Rust) and test heavily on all three OSs. Linux support is often the most fragile.
*   **Security:** Generating local IP links exposes the local network topology.
    *   *Mitigation:* This is a local-only tool; no data leaves the machine. However, users should be warned not to share the QR code publicly, as it reveals the local IP address.
