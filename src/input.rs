use evdev::{EventType, InputDevice, Key};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub fn get_key_code(name: &str) -> Option<u16> {
    KEY_NAME_TO_CODE.get(name).copied()
}

pub fn find_keyboards() -> anyhow::Result<Vec<InputDevice>> {
    let mut keyboards = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let name = entry.file_name().to_str()?;
        if !name.starts_with("event") {
            continue;
        }
        let path = Path::new("/dev/input").join(name);
        let device = match evdev::Device::open(&path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("could not open {}: {}", path.display(), e);
                continue;
            }
        };
        let has_key = device
            .supported_keys()
            .map(|k| k.contains(Key::KEY_SCROLLLOCK))
            .unwrap_or(false);
        if has_key {
            keyboards.push(device);
        }
    }
    Ok(keyboards)
}

pub struct KeyEvent {
    pub code: u16,
    pub device_idx: usize,
}

pub struct DeviceHandle {
    pub device: InputDevice,
    pub tx: Sender<KeyEvent>,
}

pub fn spawn_keyboard_reader(
    device: InputDevice,
    device_idx: usize,
) -> (DeviceHandle, Receiver<KeyEvent>) {
    let (tx, rx) = mpsc::channel();
    let dev = device.try_clone().unwrap();
    thread::spawn(move || {
        let mut events = match dev.fetch_events() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("error reading device {}: {}", dev.name(), e);
                return;
            }
        };
        loop {
            match events.next() {
                Some(Ok(event)) => {
                    if event.event_type() == EventType::KEY && event.value() == 1 {
                        let _ = tx.send(KeyEvent {
                            code: event.code(),
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
        log::info!("keyboard thread exiting: {}", dev.name());
    });
    (
        DeviceHandle { device, tx },
        rx,
    )
}

pub fn key_name(code: u16) -> Option<String> {
    KEY_CODE_TO_NAME.get(&code).map(|s| s.to_string())
}

const KEY_SCROLLLOCK: u16 = 70;
const KEY_PAUSE: u16 = 119;
const KEY_HOME: u16 = 102;
const KEY_END: u16 = 107;
const KEY_PAGEUP: u16 = 104;
const KEY_PAGEDOWN: u16 = 109;
const KEY_INSERT: u16 = 110;
const KEY_DELETE: u16 = 111;
const KEY_F1: u16 = 59;
const KEY_F2: u16 = 60;
const KEY_F3: u16 = 61;
const KEY_F4: u16 = 62;
const KEY_F5: u16 = 63;
const KEY_F6: u16 = 64;
const KEY_F7: u16 = 65;
const KEY_F8: u16 = 66;
const KEY_F9: u16 = 67;
const KEY_F10: u16 = 68;
const KEY_F11: u16 = 87;
const KEY_F12: u16 = 88;
const KEY_F13: u16 = 0x68;
const KEY_F14: u16 = 0x69;
const KEY_F15: u16 = 0x6a;
const KEY_F16: u16 = 0x6b;
const KEY_F17: u16 = 0x6c;
const KEY_F18: u16 = 0x6d;
const KEY_F19: u16 = 0x6e;
const KEY_F20: u16 = 0x6f;
const KEY_F21: u16 = 0x70;
const KEY_F22: u16 = 0x71;
const KEY_F23: u16 = 0x72;
const KEY_F24: u16 = 0x73;

static KEY_CODE_TO_NAME: Lazy<HashMap<u16, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (KEY_SCROLLLOCK, "scroll lock"),
        (KEY_PAUSE, "pause"),
        (KEY_HOME, "home"),
        (KEY_END, "end"),
        (KEY_PAGEUP, "page up"),
        (KEY_PAGEDOWN, "page down"),
        (KEY_INSERT, "insert"),
        (KEY_DELETE, "delete"),
        (KEY_F1, "f1"),
        (KEY_F2, "f2"),
        (KEY_F3, "f3"),
        (KEY_F4, "f4"),
        (KEY_F5, "f5"),
        (KEY_F6, "f6"),
        (KEY_F7, "f7"),
        (KEY_F8, "f8"),
        (KEY_F9, "f9"),
        (KEY_F10, "f10"),
        (KEY_F11, "f11"),
        (KEY_F12, "f12"),
        (KEY_F13, "f13"),
        (KEY_F14, "f14"),
        (KEY_F15, "f15"),
        (KEY_F16, "f16"),
        (KEY_F17, "f17"),
        (KEY_F18, "f18"),
        (KEY_F19, "f19"),
        (KEY_F20, "f20"),
        (KEY_F21, "f21"),
        (KEY_F22, "f22"),
        (KEY_F23, "f23"),
        (KEY_F24, "f24"),
    ])
});

static KEY_NAME_TO_CODE: Lazy<HashMap<String, u16>> = Lazy::new(|| {
    KEY_CODE_TO_NAME.iter().map(|(&k, &v)| (v.to_string(), k)).collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_code_lookups() {
        assert_eq!(get_key_code("scroll lock"), Some(KEY_SCROLLLOCK));
        assert_eq!(get_key_code("pause"), Some(KEY_PAUSE));
        assert_eq!(get_key_code("f1"), Some(KEY_F1));
        assert_eq!(get_key_code("f24"), Some(KEY_F24));
        assert_eq!(get_key_code("nonexistent"), None);
    }

    #[test]
    fn test_key_name_roundtrip() {
        for (code, name) in KEY_CODE_TO_NAME.iter() {
            assert_eq!(key_name(*code), Some(name.to_string()));
        }
    }
}