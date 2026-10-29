#!/bin/sh
# Installiert AutoHide Desktop Icons für den aktuellen Benutzer (kein sudo nötig).
# Deinstallieren: ./install.sh undo
set -e
BIN="$HOME/.local/bin/autohide-desktop-icons"
AUTOSTART="$HOME/.config/autostart/autohide-desktop-icons.desktop"

pkill -f autohide_desktop_icons.py 2>/dev/null || true
pkill -f "$BIN" 2>/dev/null || true
rm -f "$BIN" "$AUTOSTART"
if [ "$1" = undo ]; then
    echo "Entfernt."
    exit 0
fi

install -Dm755 "$(dirname "$0")/autohide_desktop_icons.py" "$BIN"
mkdir -p "$(dirname "$AUTOSTART")"
cat > "$AUTOSTART" <<DESKTOP
[Desktop Entry]
Type=Application
Name=AutoHide Desktop Icons
Comment=Blendet Desktop-Icons nach Inaktivität aus, Klick auf den Desktop zeigt sie wieder
Exec=$BIN
Icon=user-desktop
X-KDE-autostart-after=panel
NoDisplay=true
DESKTOP
setsid -f "$BIN" >/dev/null 2>&1
echo "Installiert und gestartet (startet ab jetzt bei der Anmeldung)."
