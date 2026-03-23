## Innu

Innu(INternet Network Utility) is a Rust-based Wi-Fi manager for Linux desktops that talks directly to NetworkManager over D-Bus and presents nearby networks in a focused `egui` interface. It is built for people who want a fast, native-feeling network picker that also looks great.

![Innu banner](assets/banner.jpg)

## Quick install

```bash
curl -fsSL https://raw.githubusercontent.com/gitfudge0/innu/refs/heads/main/install.sh | bash
```

### AUR

```bash
yay -S innu-bin
```

This installs the latest prebuilt release binaries system-wide.

```bash
yay -S innu-git
```

This builds Innu from source and installs the system-wide binary and desktop entry.

![Innu screenshot](assets/screenshot.png)

### Local install from a checkout

```bash
./install.sh
```

This uses the current checked-out source tree and installs the same way.

Make sure `~/.local/bin` is on your `PATH` before launching the app from a terminal.

### Developer install

```bash
cargo build --release
```

Run it directly with:

```bash
cargo run --release
```

## Requirements

- NetworkManager running on the system
- Access to the system D-Bus
- A desktop environment with Wayland or X11 support
- Rust toolchain if you are building from source

## Usage

Launch Innu from a terminal:

```bash
innu
```

## Appearance

Innu keeps the light/dark mode toggle in `~/.config/innu/theme.toml` and supports optional color/font overrides in `~/.config/innu/appearance.toml`.

`appearance.toml` is partial: any missing key falls back to the built-in theme. Only colors and the main UI text font are configurable. Layout, spacing, icon symbols, labels, and behavior do not change.

Example:

```toml
[fonts]
ui = "/absolute/path/to/font.ttf"

[light.colors]
background = "#F5F3EE"
surface = "#FFFDF8"
border = "#1F1A17"
text = "#171311"
text_muted = "#5E5750"
accent = "#B56A1E"
success = "#2F6B45"
warning = "#8A5A12"
error = "#8C2F2F"

[dark.colors]
background = "#101010"
surface = "#151515"
border = "#D6D0C7"
text = "#F2EEE7"
text_muted = "#B1AAA1"
accent = "#D0893C"
success = "#6FA57E"
warning = "#D2A45B"
error = "#D07C7C"
```

Use 6-digit hex colors. Invalid colors or unreadable font paths are ignored and fall back to the defaults.

## CLI

```bash
innu --help
innu --version
innu uninstall
```

`innu uninstall` removes a user-local install after confirmation. If Innu was installed with a package manager, it prints the removal command and optional user-data cleanup paths instead.

For package-managed installs, it prints removal guidance for both `innu-bin` and `innu-git`.

## Development

Common local commands:

```bash
cargo run
cargo test
cargo build --release
```

## Contributing

Issues and pull requests are welcome. If you want to contribute, open an issue describing the bug, UX improvement, or feature idea first when the change is substantial.
