# AutoHide Desktop Icons

**English** · [Deutsch](README.de.md)

**Hides the desktop icons on KDE Plasma 6 when you don't need them.** After 1–5 minutes without mouse or
keyboard input the icons disappear and the wallpaper is clear. A left or right click on the empty desktop
brings them back.

## Usage

Tray icon:

- **Left click:** show or hide the icons right away
- **Right click:** menu with *auto-hide* (on/off), *hide after* 1–5 min, *restart Plasmashell* and *quit*

The icons are always shown again when quitting. The interface follows the system language: German on German systems, English otherwise.

## How it works

- **Hiding:** the folder view's file filter is set via `plasmashell evaluateScript` to a pattern that
  matches no file. Icon positions are kept; a filter set beforehand is remembered and restored.
- **Inactivity:** mice and keyboards are read via evdev. Key presses only count as "activity" and are
  never stored.
- **Click on the desktop:** after a click, a short KWin script checks whether the active window is the
  desktop and no panel or popup is under the mouse pointer.

## Requirements

- KDE Plasma 6 (Wayland or X11)
- Rust toolchain (`cargo`) to build
- User in the `input` group (for evdev)

Arch / CachyOS:

```sh
sudo pacman -S --needed rust
sudo usermod -aG input $USER   # log in again afterwards
```

## Installation

```sh
git clone https://github.com/ProfessorTee/autohide-desktop-icons.git
cd autohide-desktop-icons
./install.sh
```

This installs to `~/.local/bin/autohide-desktop-icons`, adds a menu and autostart entry and starts the
tool. Uninstall: `./install.sh undo`

## Known issues

- Plasma sometimes loses the folder view (icons missing although no filter is active; the journal shows
  `FolderViewLayer.qml … Cannot read property 'model' of null`). This is a Plasma bug;
  *restart Plasmashell* in the tray menu fixes it.

## License

MIT, see [LICENSE](LICENSE).
