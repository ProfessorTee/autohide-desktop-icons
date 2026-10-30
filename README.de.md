# AutoHide Desktop Icons

[English](README.md) · **Deutsch**

**Blendet die Desktop-Icons unter KDE Plasma 6 aus, wenn du sie nicht brauchst.** Nach 1–5 Minuten
ohne Maus- oder Tastatureingabe verschwinden die Icons und das Hintergrundbild bleibt frei. Ein
Links- oder Rechtsklick auf den freien Desktop holt sie zurück.

## Bedienung

Tray-Icon:

- **Linksklick:** Icons sofort ein- oder ausblenden
- **Rechtsklick:** Menü mit *Automatisch ausblenden* (an/aus), *Ausblenden nach* 1–5 min,
  *Plasmashell neu starten* und *Beenden*

Beim Beenden werden die Icons immer wieder eingeblendet. Die Oberfläche folgt der Systemsprache: Deutsch auf deutschen Systemen, sonst Englisch.

## Wie es funktioniert

- **Ausblenden:** Der Dateifilter der Ordneransicht wird per `plasmashell evaluateScript` auf ein Muster
  gesetzt, das keine Datei trifft. Die Icon-Positionen bleiben erhalten, ein vorher gesetzter Filter wird
  gemerkt und wiederhergestellt.
- **Inaktivität:** Mäuse und Tastaturen werden per evdev gelesen. Tastendrücke zählen nur als
  „Aktivität“ und werden nie gespeichert.
- **Klick auf den Desktop:** Ein kurzes KWin-Skript prüft nach dem Klick, ob das aktive Fenster der
  Desktop ist und unter dem Mauszeiger keine Leiste und kein Popup liegt.

## Voraussetzungen

- KDE Plasma 6 (Wayland oder X11)
- Python 3 mit `PySide6` und `python-evdev`
- Benutzer in der Gruppe `input` (für evdev)

Arch / CachyOS:

```sh
sudo pacman -S --needed pyside6 python-evdev
sudo usermod -aG input $USER   # danach neu anmelden
```

## Installation

```sh
git clone https://github.com/ProfessorTee/autohide-desktop-icons.git
cd autohide-desktop-icons
./install.sh
```

Das installiert nach `~/.local/bin/autohide-desktop-icons`, legt einen Menü- und Autostart-Eintrag an und
startet das Tool. Deinstallieren: `./install.sh undo`

## Bekannte Themen

- Plasma verliert manchmal die Ordneransicht (Icons fehlen, obwohl kein Filter aktiv ist; im Journal
  steht `FolderViewLayer.qml … Cannot read property 'model' of null`). Das ist ein Plasma-Fehler;
  *Plasmashell neu starten* im Tray-Menü behebt es.

## Lizenz

MIT, siehe [LICENSE](LICENSE).
