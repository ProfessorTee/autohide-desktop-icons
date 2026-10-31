//! AutoHide Desktop Icons - blendet die KDE-Desktop-Icons nach X Minuten ohne Maus-/Tastatureingabe aus.
//! Links- oder Rechtsklick auf den freien Desktop blendet sie wieder ein. Tray-Icon: Linksklick = sofort
//! ein/aus, Rechtsklick = Menue (aktiv, Zeit 1-5 min, Plasmashell neu starten, beenden).
//!
//! Wie:
//! - Ausblenden = Dateifilter der Ordneransicht auf ein Muster setzen, das keine Datei trifft (per
//!   plasmashell evaluateScript). Icon-Positionen bleiben. Der alte Filter wird in derselben Plasma-Config
//!   gemerkt und zurueckgesetzt.
//! - Eingaben kommen per evdev (/dev/input, User muss in Gruppe "input" sein). Tastendruecke werden nur als
//!   "Aktivitaet" gezaehlt, nie gespeichert.
//! - Klick auf den Desktop erkennt ein kurzes KWin-Skript kurz nach dem Klick: aktives Fenster == Desktop
//!   und unter dem Mauszeiger keine Leiste/Popup -> Icons wieder einblenden. Ueber das aktive Fenster statt
//!   nur ueber die Stapelreihenfolge, weil "Desktop anzeigen" Fenster (z.B. Vollbildspiele) nur versteckt,
//!   sie bleiben im Stapel ueber dem Desktop.
use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{LazyLock, mpsc};
use std::time::{Duration, Instant};

// German UI on a German system, English otherwise.
static DE: LazyLock<bool> = LazyLock::new(|| {
    ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .is_some_and(|v| v.starts_with("de"))
});

fn t(de: &'static str, en: &'static str) -> &'static str {
    if *DE { de } else { en }
}

const MARKER: &str = "autohide-desktop-icons-none";

// Laeuft in plasmashell. Nur echte Desktops (Ordneransicht) mit Bildschirm.
fn desktops(body: &str) -> String {
    format!(r#"for (const d of desktops()) {{ if (d.screen < 0 || d.type != "org.kde.plasma.folder") continue; d.currentConfigGroup = ["General"]; {body} }}"#)
}

static JS_HIDE: LazyLock<String> = LazyLock::new(|| desktops(&format!(
    r#"if (d.readConfig("filterPattern") != "{MARKER}") {{ d.writeConfig("autohideOrigMode", d.readConfig("filterMode")); d.writeConfig("autohideOrigPattern", d.readConfig("filterPattern")); }} d.writeConfig("filterMode", 1); d.writeConfig("filterPattern", "{MARKER}"); d.reloadConfig();"#)));
static JS_SHOW: LazyLock<String> = LazyLock::new(|| desktops(&format!(
    r#"if (d.readConfig("filterPattern") == "{MARKER}") {{ d.writeConfig("filterMode", d.readConfig("autohideOrigMode") || 0); d.writeConfig("filterPattern", d.readConfig("autohideOrigPattern") || ""); d.reloadConfig(); }}"#)));
static JS_STATE: LazyLock<String> = LazyLock::new(|| desktops(&format!(r#"if (d.readConfig("filterPattern") == "{MARKER}") print("hidden");"#)));

// Laeuft in KWin: Desktop unter dem Mauszeiger? -> JS_SHOW an plasmashell.
fn kwin_probe_js() -> String {
    let show = format!("\"{}\"", JS_SHOW.replace('\\', "\\\\").replace('"', "\\\""));
    format!(r#"
const a = workspace.activeWindow, p = workspace.cursorPos, st = workspace.stackingOrder;
let onDesktop = a && a.desktopWindow;
for (let i = st.length - 1; onDesktop && i >= 0; i--) {{
    const w = st[i], g = w.frameGeometry;
    if (w.minimized || w.hidden || !(w.onAllDesktops || w.desktops.includes(workspace.currentDesktop))) continue;
    if (p.x < g.x || p.x >= g.x + g.width || p.y < g.y || p.y >= g.y + g.height) continue;
    if (w.desktopWindow) break;
    if (w.dock || w.popupWindow || w.notification || w.onScreenDisplay) onDesktop = false;  // Leiste, Menue, Popup
}}
if (onDesktop)
    callDBus("org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell", "evaluateScript", {show});
"#)
}

static PROBE_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("autohide-desktop-icons-probe.js")
});

static START: LazyLock<Instant> = LazyLock::new(Instant::now);
static LAST_ACTIVITY: AtomicU64 = AtomicU64::new(0); // ms seit START
static STOP: AtomicBool = AtomicBool::new(false);

fn touch() {
    LAST_ACTIVITY.store(START.elapsed().as_millis() as u64, Ordering::Relaxed);
}

fn idle() -> Duration {
    START.elapsed().saturating_sub(Duration::from_millis(LAST_ACTIVITY.load(Ordering::Relaxed)))
}

struct Bus(zbus::blocking::Connection);

impl Bus {
    fn plasma(&self, js: &str) -> String {
        self.0.call_method(Some("org.kde.plasmashell"), "/PlasmaShell", Some("org.kde.PlasmaShell"), "evaluateScript", &(js,))
            .and_then(|m| m.body().deserialize::<String>()).unwrap_or_default()
    }

    fn kwin_probe(&self) {
        let call = |method: &str| self.0.call_method(Some("org.kde.KWin"), "/Scripting", Some("org.kde.kwin.Scripting"),
                                                     method, &("autohide-desktop-icons",));
        let _ = call("unloadScript");
        let id = self.0.call_method(Some("org.kde.KWin"), "/Scripting", Some("org.kde.kwin.Scripting"), "loadScript",
                                    &(PROBE_FILE.to_string_lossy().as_ref(), "autohide-desktop-icons"))
            .and_then(|m| m.body().deserialize::<i32>());
        if let Ok(id @ 0..) = id {
            let _ = self.0.call_method(Some("org.kde.KWin"), format!("/Scripting/Script{id}").as_str(),
                                       Some("org.kde.kwin.Script"), "run", &());
        }
        let _ = call("unloadScript");
    }
}

/// QSettings-Datei der Python-Version, damit Einstellungen erhalten bleiben.
struct Settings {
    enabled: bool,
    timeout_min: u64,
}

fn settings_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/autohide-desktop-icons/autohide-desktop-icons.conf")
}

impl Settings {
    fn load() -> Self {
        let text = std::fs::read_to_string(settings_path()).unwrap_or_default();
        let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=').map(str::trim));
        Settings { enabled: get("enabled") != Some("false"), timeout_min: get("timeout_min").and_then(|v| v.parse().ok()).unwrap_or(2) }
    }

    fn save(&self) {
        let path = settings_path();
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(path, format!("[General]\nenabled={}\ntimeout_min={}\n", self.enabled, self.timeout_min));
    }
}

/// Liest Maeuse + Tastaturen per evdev (Hotplug: Rescan alle 3 s); Links-/Rechtsklicks gehen an `clicks`.
fn watch_input(clicks: mpsc::Sender<Msg>) {
    const BTN_LEFT: u16 = 0x110;
    const BTN_RIGHT: u16 = 0x111;
    const KEY_A: usize = 30;
    let wanted = |node: &str| {
        let words: Vec<u64> = std::fs::read_to_string(format!("/sys/class/input/{node}/device/capabilities/key"))
            .unwrap_or_default().split_whitespace().map(|w| u64::from_str_radix(w, 16).unwrap_or(0)).collect();
        let bit = |n: usize| words.len() > n / 64 && words[words.len() - 1 - n / 64] >> (n % 64) & 1 == 1;
        bit(BTN_LEFT as usize) || bit(KEY_A)
    };
    let mut devices: Vec<(String, File)> = vec![];
    let mut known: Vec<String> = vec![];
    let mut next_scan = Instant::now();
    let mut buf = [0u8; 24 * 64]; // struct input_event: timeval, u16 type, u16 code, i32 value
    loop {
        if Instant::now() >= next_scan {
            next_scan = Instant::now() + Duration::from_secs(3);
            let nodes: Vec<String> = std::fs::read_dir("/dev/input").into_iter().flatten().flatten()
                .map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("event")).collect();
            for node in nodes.iter().filter(|n| !known.contains(n)) {
                if wanted(node) {
                    let f = std::fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(format!("/dev/input/{node}"));
                    if let Ok(f) = f {
                        devices.push((node.clone(), f));
                    }
                }
            }
            known = nodes;
        }
        let mut fds: Vec<libc::pollfd> = devices.iter().map(|(_, f)| libc::pollfd { fd: f.as_raw_fd(), events: libc::POLLIN, revents: 0 }).collect();
        unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 1000) };
        let mut gone = vec![];
        for (i, fd) in fds.iter().enumerate() {
            if fd.revents & (libc::POLLERR | libc::POLLHUP) != 0 {
                gone.push(i); // Geraet abgezogen
            } else if fd.revents & libc::POLLIN != 0 {
                let Ok(len) = devices[i].1.read(&mut buf) else { gone.push(i); continue };
                touch();
                for ev in buf[..len].chunks_exact(24) {
                    let (typ, code) = (u16::from_ne_bytes([ev[16], ev[17]]), u16::from_ne_bytes([ev[18], ev[19]]));
                    let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
                    if typ == 1 && value == 1 && (code == BTN_LEFT || code == BTN_RIGHT) {
                        let _ = clicks.send(Msg::Click);
                    }
                }
            }
        }
        for i in gone.into_iter().rev() {
            let (node, _) = devices.remove(i);
            known.retain(|n| *n != node);
        }
    }
}

enum Msg {
    Click,
    Toggle,
    Enabled(bool),
    Timeout(u64),
    Quit,
}

struct Tray {
    hidden: bool,
    enabled: bool,
    timeout_min: u64,
    tx: mpsc::Sender<Msg>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "autohide-desktop-icons".into()
    }

    fn title(&self) -> String {
        "AutoHide Desktop Icons".into()
    }

    fn icon_name(&self) -> String {
        if self.hidden { "view-hidden" } else { "user-desktop" }.into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let state = if self.hidden { t("ausgeblendet", "hidden") } else { t("sichtbar", "visible") };
        ksni::ToolTip { title: format!("{}{state}", t("Desktop-Icons ", "Desktop icons ")), ..Default::default() }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(Msg::Toggle);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let enabled = self.enabled;
        vec![
            CheckmarkItem { label: t("Automatisch ausblenden", "Auto-hide").into(), checked: enabled,
                            activate: Box::new(move |this: &mut Self| { let _ = this.tx.send(Msg::Enabled(!enabled)); }),
                            ..Default::default() }.into(),
            SubMenu { label: t("Ausblenden nach", "Hide after").into(), submenu: vec![RadioGroup {
                selected: self.timeout_min.saturating_sub(1) as usize,
                select: Box::new(|this: &mut Self, i| { let _ = this.tx.send(Msg::Timeout(i as u64 + 1)); }),
                options: (1..=5).map(|m| RadioItem { label: format!("{m} min"), ..Default::default() }).collect(),
            }.into()], ..Default::default() }.into(),
            MenuItem::Separator,
            StandardItem { label: if self.hidden { t("Icons einblenden", "Show icons") } else { t("Icons ausblenden", "Hide icons") }.into(),
                           activate: Box::new(|this: &mut Self| { let _ = this.tx.send(Msg::Toggle); }), ..Default::default() }.into(),
            // Plasma verliert manchmal die Ordneransicht (Icons weg trotz Filter aus) -> Neustart hilft
            StandardItem { label: t("Plasmashell neu starten", "Restart Plasmashell").into(),
                           activate: Box::new(|_| { let _ = Command::new("systemctl").args(["--user", "restart", "plasma-plasmashell.service"]).spawn(); }),
                           ..Default::default() }.into(),
            StandardItem { label: t("Beenden", "Quit").into(), activate: Box::new(|this: &mut Self| { let _ = this.tx.send(Msg::Quit); }),
                           ..Default::default() }.into(),
        ]
    }
}

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

fn main() {
    use ksni::blocking::TrayMethods;
    let Ok(conn) = zbus::blocking::Connection::session() else {
        eprintln!("{}", t("Keine D-Bus-Session", "No D-Bus session"));
        std::process::exit(1);
    };
    let bus = Bus(conn);
    let _ = std::fs::write(&*PROBE_FILE, kwin_probe_js());
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
    }
    let mut settings = Settings::load();
    let (tx, rx) = mpsc::channel();
    touch();
    let click_tx = tx.clone();
    std::thread::spawn(move || watch_input(click_tx));

    bus.plasma(&JS_SHOW); // Reste eines Absturzes aufraeumen
    let mut hidden = false;
    let handle = Tray { hidden, enabled: settings.enabled, timeout_min: settings.timeout_min, tx }.spawn().expect("tray");
    let (mut probe_at, mut refresh_at): (Option<Instant>, Option<Instant>) = (None, None);
    let mut check_at = Instant::now() + Duration::from_secs(2);
    let mut shown = (hidden, settings.enabled, settings.timeout_min); // what the tray displays
    while !STOP.load(Ordering::Relaxed) {
        let next = [probe_at, refresh_at, Some(check_at)].into_iter().flatten().min().unwrap();
        let wait = next.saturating_duration_since(Instant::now()).min(Duration::from_millis(500)); // STOP pruefen
        let mut refresh = false;
        let mut set = None;
        match rx.recv_timeout(wait) {
            Ok(Msg::Click) if hidden => {
                // erst nachdem KWin den Klick-Fokus gesetzt hat
                probe_at = Some(Instant::now() + Duration::from_millis(150));
                refresh_at = Some(Instant::now() + Duration::from_millis(450));
            }
            Ok(Msg::Click) => {}
            Ok(Msg::Toggle) => set = Some(!hidden),
            Ok(Msg::Enabled(on)) => {
                settings.enabled = on;
                settings.save();
            }
            Ok(Msg::Timeout(m)) => {
                settings.timeout_min = m;
                settings.save();
            }
            Ok(Msg::Quit) => break,
            Err(_) => {}
        }
        let now = Instant::now();
        if probe_at.is_some_and(|t| t <= now) {
            probe_at = None;
            bus.kwin_probe();
        }
        if refresh_at.is_some_and(|t| t <= now) {
            refresh_at = None;
            refresh = true;
        }
        if check_at <= now {
            check_at = now + Duration::from_secs(2);
            refresh |= hidden; // evtl. extern wieder eingeblendet (Klick, Plasma-Neustart)
            if !hidden && settings.enabled && idle() >= Duration::from_secs(settings.timeout_min * 60) {
                set = Some(true);
            }
        }
        if let Some(hide) = set {
            bus.plasma(if hide { &JS_HIDE } else { &JS_SHOW });
            touch();
            refresh = true;
        }
        if refresh {
            hidden = bus.plasma(&JS_STATE).contains("hidden");
        }
        let state = (hidden, settings.enabled, settings.timeout_min);
        if state != shown {
            shown = state;
            handle.update(move |t: &mut Tray| (t.hidden, t.enabled, t.timeout_min) = state);
        }
    }
    bus.plasma(&JS_SHOW); // Icons nie versteckt zuruecklassen
}
