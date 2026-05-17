use evdev::Key;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub fn get_key_code(name: &str) -> Option<u16> {
    KEY_NAME_TO_CODE.get(name).copied()
}

pub fn find_keyboards() -> anyhow::Result<Vec<evdev::Device>> {
    let mut keyboards = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let name = entry.file_name().to_str()?;
        if !name.starts_with("event") {
            continue;
        }
        let path = Path::new("/dev/input").join(name);
        let mut device = match evdev::Device::open(&path) {
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
        if supported.contains(Key::KEY_SCROLLLOCK) {
            keyboards.push(device);
        }
    }
    Ok(keyboards)
}

pub struct KeyEvent {
    pub code: u16,
    pub value: i32,
    pub device_idx: usize,
}

pub struct DeviceHandle {
    pub device: evdev::Device,
    pub tx: Sender<KeyEvent>,
}

pub fn spawn_keyboard_reader(
    mut device: evdev::Device,
    device_idx: usize,
) -> (DeviceHandle, Receiver<KeyEvent>) {
    let (tx, rx) = mpsc::channel();
    let dev = device.take().unwrap();
    thread::spawn(move || {
        let mut events = match dev.fetch_events() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("error reading device {}: {}", dev.name().unwrap_or("?"), e);
                return;
            }
        };
        loop {
            match events.next() {
                Some(Ok(event)) => {
                    if event.event_type() == evdev::EventType::KEY && event.value() == 1 {
                        let _ = tx.send(KeyEvent {
                            code: event.code(),
                            value: event.value(),
                            device_idx,
                        });
                    }
                }
                Some(Err(e)) => {
                    log::warn!("event error: {}", e);
                    break;
                }
                None => {
                    break;
                }
            }
        }
        log::info!("keyboard thread exiting");
    });
    (
        DeviceHandle { device, tx },
        rx,
    )
}

pub fn key_name(code: u16) -> Option<String> {
    KEY_CODE_TO_NAME.get(&code).map(|s| s.to_string())
}

static KEY_CODE_TO_NAME: Lazy<HashMap<u16, &'static str>> = Lazy::new(|| {
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

static KEY_NAME_TO_CODE: Lazy<HashMap<String, u16>> =
    Lazy::new(|| KEY_CODE_TO_NAME.iter().map(|(&k, &v)| (v.to_string(), k)).collect());

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