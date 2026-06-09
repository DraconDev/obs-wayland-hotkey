use evdev::{Device, KeyCode};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::LazyLock;
use std::thread;

pub fn get_key_code(name: &str) -> Option<u16> {
    KEY_NAME_TO_CODE.get(&normalize_key_name(name)).copied()
}

pub fn normalize_key_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut last_was_space = false;

    for ch in name.trim().chars() {
        if ch == '_' {
            if !last_was_space {
                normalized.push(' ');
                last_was_space = true;
            }
            continue;
        }

        let ch = ch.to_ascii_lowercase();
        if ch.is_whitespace() {
            if !last_was_space {
                normalized.push(' ');
                last_was_space = true;
            }
        } else {
            normalized.push(ch);
            last_was_space = false;
        }
    }

    normalized.trim().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    tokens: Vec<KeyToken>,
}

impl KeyChord {
    pub fn parse(spec: &str) -> anyhow::Result<Self> {
        let normalized = normalize_key_name(spec);
        if normalized.is_empty() {
            return Ok(Self { tokens: Vec::new() });
        }

        let parts: Vec<&str> = normalized
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            anyhow::bail!("empty key chord");
        }

        let mut tokens = Vec::with_capacity(parts.len());
        let mut seen = HashSet::new();
        for part in parts {
            let token = KEY_TOKENS
                .iter()
                .find(|token| token.matches_name(part))
                .ok_or_else(|| anyhow::anyhow!("unknown key '{part}'"))?;

            if seen.insert(token.canonical_name()) {
                tokens.push(token.clone());
            }
        }

        Ok(Self { tokens })
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn matches(&self, pressed: &HashSet<u16>) -> bool {
        self.tokens
            .iter()
            .all(|token| token.matches_any_pressed(pressed))
    }

    pub fn display(&self) -> String {
        self.tokens
            .iter()
            .map(KeyToken::canonical_name)
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyToken {
    names: Vec<&'static str>,
    codes: Vec<u16>,
}

impl KeyToken {
    fn new(names: &[&'static str], codes: &[KeyCode]) -> Self {
        Self {
            names: names.to_vec(),
            codes: codes.iter().map(|code| code.code()).collect(),
        }
    }

    fn matches_name(&self, name: &str) -> bool {
        self.names.iter().any(|known| *known == name)
    }

    fn matches_any_pressed(&self, pressed: &HashSet<u16>) -> bool {
        self.codes.iter().any(|code| pressed.contains(code))
    }

    fn canonical_name(&self) -> &'static str {
        self.names[0]
    }
}

pub fn find_keyboards() -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.starts_with("event") {
            continue;
        }
        let path = PathBuf::from("/dev/input").join(&name);
        let device = match Device::open(&path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("could not open {}: {}", path.display(), e);
                continue;
            }
        };
        let supported = match device.supported_keys() {
            Some(k) => k,
            None => continue,
        };
        // Detect keyboards by checking for common typing keys rather than
        // KEY_SCROLLLOCK, which many keyboards (laptops, compact boards) do
        // not advertise in their evdev capability bitmap.
        if supported.contains(KeyCode::KEY_A)
            && supported.contains(KeyCode::KEY_SPACE)
            && supported.contains(KeyCode::KEY_ENTER)
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub struct KeyEvent {
    pub code: u16,
    pub value: i32,
}

pub struct DeviceHandle {
    #[allow(dead_code)]
    pub path: PathBuf,
    tx: Sender<()>,
}

pub fn spawn_keyboard_reader(
    path: PathBuf,
    _device_idx: usize,
) -> (DeviceHandle, Receiver<KeyEvent>) {
    let (close_tx, close_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let path_clone = path.clone();

    thread::spawn(move || {
        let mut device = match Device::open(&path_clone) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("could not open {}: {}", path_clone.display(), e);
                return;
            }
        };
        let name = device.name().unwrap_or("?").to_string();
        log::info!(
            "keyboard thread started: {} at {}",
            name,
            path_clone.display()
        );

        // Use recv_timeout so the loop periodically checks close_rx.
        // This avoids blocking indefinitely in fetch_events().
        const TIMEOUT_MS: u32 = 500;

        loop {
            // Check for shutdown signal first
            match close_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT_MS as u64)) {
                Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Timeout is fine — keep polling the device
                }
            }

            let events = match device.fetch_events() {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("error reading device {}: {}", name, e);
                    break;
                }
            };

            for event in events {
                if event.event_type() == evdev::EventType::KEY
                    && (event.value() == 0 || event.value() == 1)
                {
                    let _ = tx.send(KeyEvent {
                        code: event.code(),
                        value: event.value(),
                    });
                }
            }
        }
        log::info!("keyboard thread exiting: {}", name);
    });

    (DeviceHandle { path, tx: close_tx }, rx)
}

#[allow(dead_code)]
pub fn key_name(code: u16) -> Option<String> {
    KEY_CODE_TO_NAME.get(&code).map(|s| s.to_string())
}

static KEY_TOKENS: LazyLock<Vec<KeyToken>> = LazyLock::new(|| {
    vec![
        KeyToken::new(&["left ctrl"], &[KeyCode::KEY_LEFTCTRL]),
        KeyToken::new(
            &["ctrl", "control"],
            &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_RIGHTCTRL],
        ),
        KeyToken::new(&["left shift"], &[KeyCode::KEY_LEFTSHIFT]),
        KeyToken::new(
            &["shift"],
            &[KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_RIGHTSHIFT],
        ),
        KeyToken::new(&["left alt"], &[KeyCode::KEY_LEFTALT]),
        KeyToken::new(
            &["alt", "option"],
            &[KeyCode::KEY_LEFTALT, KeyCode::KEY_RIGHTALT],
        ),
        KeyToken::new(&["left super"], &[KeyCode::KEY_LEFTMETA]),
        KeyToken::new(
            &["super", "command", "win"],
            &[KeyCode::KEY_LEFTMETA, KeyCode::KEY_RIGHTMETA],
        ),
        KeyToken::new(&["left meta"], &[KeyCode::KEY_LEFTMETA]),
        KeyToken::new(&["meta"], &[KeyCode::KEY_LEFTMETA, KeyCode::KEY_RIGHTMETA]),
        KeyToken::new(&["caps lock"], &[KeyCode::KEY_CAPSLOCK]),
        KeyToken::new(&["scroll lock"], &[KeyCode::KEY_SCROLLLOCK]),
        KeyToken::new(&["pause"], &[KeyCode::KEY_PAUSE]),
        KeyToken::new(&["home"], &[KeyCode::KEY_HOME]),
        KeyToken::new(&["end"], &[KeyCode::KEY_END]),
        KeyToken::new(&["page up"], &[KeyCode::KEY_PAGEUP]),
        KeyToken::new(&["page down"], &[KeyCode::KEY_PAGEDOWN]),
        KeyToken::new(&["insert"], &[KeyCode::KEY_INSERT]),
        KeyToken::new(&["delete"], &[KeyCode::KEY_DELETE]),
        KeyToken::new(&["f1"], &[KeyCode::KEY_F1]),
        KeyToken::new(&["f2"], &[KeyCode::KEY_F2]),
        KeyToken::new(&["f3"], &[KeyCode::KEY_F3]),
        KeyToken::new(&["f4"], &[KeyCode::KEY_F4]),
        KeyToken::new(&["f5"], &[KeyCode::KEY_F5]),
        KeyToken::new(&["f6"], &[KeyCode::KEY_F6]),
        KeyToken::new(&["f7"], &[KeyCode::KEY_F7]),
        KeyToken::new(&["f8"], &[KeyCode::KEY_F8]),
        KeyToken::new(&["f9"], &[KeyCode::KEY_F9]),
        KeyToken::new(&["f10"], &[KeyCode::KEY_F10]),
        KeyToken::new(&["f11"], &[KeyCode::KEY_F11]),
        KeyToken::new(&["f12"], &[KeyCode::KEY_F12]),
        KeyToken::new(&["f13"], &[KeyCode::KEY_F13]),
        KeyToken::new(&["f14"], &[KeyCode::KEY_F14]),
        KeyToken::new(&["f15"], &[KeyCode::KEY_F15]),
        KeyToken::new(&["f16"], &[KeyCode::KEY_F16]),
        KeyToken::new(&["f17"], &[KeyCode::KEY_F17]),
        KeyToken::new(&["f18"], &[KeyCode::KEY_F18]),
        KeyToken::new(&["f19"], &[KeyCode::KEY_F19]),
        KeyToken::new(&["f20"], &[KeyCode::KEY_F20]),
        KeyToken::new(&["f21"], &[KeyCode::KEY_F21]),
        KeyToken::new(&["f22"], &[KeyCode::KEY_F22]),
        KeyToken::new(&["f23"], &[KeyCode::KEY_F23]),
        KeyToken::new(&["f24"], &[KeyCode::KEY_F24]),
    ]
});

static KEY_CODE_TO_NAME: LazyLock<HashMap<u16, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for token in KEY_TOKENS.iter() {
        for code in &token.codes {
            map.entry(*code).or_insert(token.canonical_name());
        }
    }
    map
});

static KEY_NAME_TO_CODE: LazyLock<HashMap<String, u16>> = LazyLock::new(|| {
    KEY_CODE_TO_NAME
        .iter()
        .map(|(&k, &v)| (v.to_string(), k))
        .collect()
});

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_code_lookups() {
        assert_eq!(get_key_code("scroll lock"), Some(70));
        assert_eq!(get_key_code("pause"), Some(119));
        assert_eq!(get_key_code("f1"), Some(59));
        assert_eq!(get_key_code("f24"), Some(194));
        assert_eq!(get_key_code("nonexistent"), None);
    }

    #[test]
    fn test_normalize_key_name() {
        assert_eq!(normalize_key_name("  Scroll_Lock  "), "scroll lock");
        assert_eq!(normalize_key_name("ctrl+shift+f1"), "ctrl+shift+f1");
    }

    #[test]
    fn test_parse_key_chord_single_key() {
        let chord = KeyChord::parse("f1").unwrap();
        assert_eq!(chord.display(), "f1");
        assert!(chord.matches(&HashSet::from([59])));
        assert!(!chord.matches(&HashSet::new()));
    }

    #[test]
    fn test_parse_key_chord_with_aliases() {
        let chord = KeyChord::parse("ctrl + shift + f1").unwrap();
        assert_eq!(chord.display(), "ctrl + shift + f1");
        assert!(chord.matches(&HashSet::from([29, 42, 59])));
        assert!(chord.matches(&HashSet::from([97, 54, 59])));
        assert!(!chord.matches(&HashSet::from([29, 59])));
    }

    #[test]
    fn test_parse_key_chord_unknown_key() {
        let result = KeyChord::parse("ctrl + nope");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_key_chord_deduplicates_repeated_key() {
        let chord = KeyChord::parse("left ctrl + left ctrl + f1").unwrap();
        assert_eq!(chord.display(), "left ctrl + f1");
    }

    #[test]
    fn test_key_name_roundtrip() {
        for (code, name) in KEY_CODE_TO_NAME.iter() {
            assert_eq!(key_name(*code), Some(name.to_string()));
        }
    }
}
