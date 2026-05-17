use evdev::{Device, KeyCode};
use std::sync::LazyLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub fn get_key_code(name: &str) -> Option<u16> {
    KEY_NAME_TO_CODE.get(name).copied()
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
        if supported.contains(KeyCode::KEY_SCROLLLOCK) {
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
        log::info!("keyboard thread started: {} at {}", name, path_clone.display());

        loop {
            let events = match device.fetch_events() {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("error reading device {}: {}", name, e);
                    break;
                }
            };

            if close_rx.try_recv().is_ok() {
                break;
            }

            for event in events {
                if event.event_type() == evdev::EventType::KEY && event.value() == 1 {
                    let _ = tx.send(KeyEvent {
                        code: event.code(),
                        value: event.value(),
                    });
                }
            }
        }
        log::info!("keyboard thread exiting: {}", name);
    });

    (
        DeviceHandle { path, tx: close_tx },
        rx,
    )
}

#[allow(dead_code)]
pub fn key_name(code: u16) -> Option<String> {
    KEY_CODE_TO_NAME.get(&code).map(|s| s.to_string())
}

static KEY_CODE_TO_NAME: LazyLock<HashMap<u16, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (70, "scroll lock"),
        (119, "pause"),
        (102, "home"),
        (107, "end"),
        (104, "page up"),
        (109, "page down"),
        (110, "insert"),
        (111, "delete"),
        (59, "f1"),
        (60, "f2"),
        (61, "f3"),
        (62, "f4"),
        (63, "f5"),
        (64, "f6"),
        (65, "f7"),
        (66, "f8"),
        (67, "f9"),
        (68, "f10"),
        (87, "f11"),
        (88, "f12"),
        (183, "f13"),
        (184, "f14"),
        (185, "f15"),
        (186, "f16"),
        (187, "f17"),
        (188, "f18"),
        (189, "f19"),
        (190, "f20"),
        (191, "f21"),
        (192, "f22"),
        (193, "f23"),
        (194, "f24"),
    ])
});

static KEY_NAME_TO_CODE: LazyLock<HashMap<String, u16>> =
    LazyLock::new(|| KEY_CODE_TO_NAME.iter().map(|(&k, &v)| (v.to_string(), k)).collect());

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
    fn test_key_name_roundtrip() {
        for (code, name) in KEY_CODE_TO_NAME.iter() {
            assert_eq!(key_name(*code), Some(name.to_string()));
        }
    }
}