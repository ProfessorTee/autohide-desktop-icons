#!/bin/sh
# Installiert AutoHide Desktop Icons für den aktuellen Benutzer (kein sudo nötig).
# Deinstallieren: ./install.sh undo
set -e
BIN="$HOME/.local/bin/autohide-desktop-icons"
AUTOSTART="$HOME/.config/autostart/autohide-desktop-icons.desktop"
MENU="$HOME/.local/share/applications/autohide-desktop-icons.desktop"

pkill -f autohide_desktop_icons.py 2>/dev/null || true  # old Python version
pkill -x autohide-deskto 2>/dev/null || true  # comm is cut to 15 chars
pkill -f "python3 $BIN" 2>/dev/null || true
rm -f "$BIN" "$AUTOSTART" "$MENU"
if [ "$1" = undo ]; then
    echo "Removed."
    exit 0
fi

cargo build --release --manifest-path "$(dirname "$0")/Cargo.toml"
install -Dm755 "$(dirname "$0")/target/release/autohide-desktop-icons" "$BIN"
mkdir -p "$(dirname "$AUTOSTART")" "$(dirname "$MENU")"
cat > "$MENU" <<DESKTOP
[Desktop Entry]
Type=Application
Name=AutoHide Desktop Icons
Comment=Hides desktop icons after inactivity, a click on the desktop shows them again
Comment[de]=Blendet Desktop-Icons nach Inaktivität aus, Klick auf den Desktop zeigt sie wieder
Exec=$BIN
Icon=user-desktop
Categories=Utility;
X-KDE-autostart-after=panel
DESKTOP
cp "$MENU" "$AUTOSTART"
setsid -f "$BIN" >/dev/null 2>&1
echo "Installed and started (starts at login from now on)."
