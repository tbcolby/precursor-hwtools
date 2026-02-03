use std::io::{Read, Seek, SeekFrom, Write};

const DICT_NAME: &str = "hwtools.settings";
const KEY_NAME: &str = "config";

#[derive(Debug, Clone)]
pub struct Settings {
    pub last_tab: u8,
    pub gpio_directions: u8,
    pub gpio_outputs: u8,
    pub uart_mux: u8,
    pub auto_refresh: bool,
    pub refresh_interval_ms: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            last_tab: 0,
            gpio_directions: 0x00, // all inputs by default
            gpio_outputs: 0x00,
            uart_mux: 1, // Log by default
            auto_refresh: true,
            refresh_interval_ms: 2000,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let pddb = pddb::Pddb::new();

        match pddb.get(DICT_NAME, KEY_NAME, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut buf = Vec::new();
                if key.read_to_end(&mut buf).is_ok() && buf.len() >= 10 {
                    Settings {
                        last_tab: buf[0],
                        gpio_directions: buf[1],
                        gpio_outputs: buf[2],
                        uart_mux: buf[3],
                        auto_refresh: buf[4] != 0,
                        refresh_interval_ms: u32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]),
                    }
                } else {
                    log::info!("Settings data too short, using defaults");
                    Settings::default()
                }
            }
            Err(e) => {
                log::info!("No settings found ({:?}), using defaults", e);
                Settings::default()
            }
        }
    }

    pub fn save(&self) {
        let pddb = pddb::Pddb::new();

        let mut data = Vec::with_capacity(16);
        data.push(self.last_tab);
        data.push(self.gpio_directions);
        data.push(self.gpio_outputs);
        data.push(self.uart_mux);
        data.push(if self.auto_refresh { 1 } else { 0 });
        data.extend_from_slice(&self.refresh_interval_ms.to_le_bytes());

        match pddb.get(
            DICT_NAME,
            KEY_NAME,
            None,
            true,  // create dict
            true,  // create key
            Some(data.len()),
            None::<fn()>,
        ) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                if let Err(e) = key.write_all(&data) {
                    log::warn!("Failed to write settings: {:?}", e);
                }
                if let Err(e) = pddb.sync() {
                    log::warn!("Failed to sync PDDB: {:?}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to open settings key for writing: {:?}", e);
            }
        }
    }
}
