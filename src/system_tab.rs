use crate::ui;
use com::{BattStats, Com};
use gam::menu::*;
use gam::{Gam, GlyphStyle};

pub struct SystemTab {
    com: Com,
    llio: llio::Llio,

    // Cached data
    batt_stats: Option<BattStats>,
    charging: Option<bool>,
    ec_uptime: Option<u64>,
    ec_git_rev: Option<(u32, bool)>,
    ec_sw_tag: Option<xous_semver::SemVer>,
    soc_git_rev: Option<xous_semver::SemVer>,
    soc_dna: Option<u64>,
    adc_vbus: Option<u16>,
    adc_temp: Option<u16>,
    adc_vccint: Option<u16>,
    adc_vccaux: Option<u16>,
    gyro: Option<(u16, u16, u16)>,
}

impl SystemTab {
    pub fn new(xns: &xous_names::XousNames) -> Self {
        let com = Com::new(xns).expect("can't connect to COM");
        let llio = llio::Llio::new(xns);

        SystemTab {
            com,
            llio,
            batt_stats: None,
            charging: None,
            ec_uptime: None,
            ec_git_rev: None,
            ec_sw_tag: None,
            soc_git_rev: None,
            soc_dna: None,
            adc_vbus: None,
            adc_temp: None,
            adc_vccint: None,
            adc_vccaux: None,
            gyro: None,
        }
    }

    pub fn refresh(&mut self) {
        // Fetch battery stats
        self.batt_stats = self.com.get_batt_stats_blocking().ok();

        // Fetch charging status
        self.charging = self.com.is_charging().ok();

        // Fetch EC uptime
        self.ec_uptime = self.com.get_ec_uptime().ok();

        // Fetch EC git rev
        self.ec_git_rev = self.com.get_ec_git_rev().ok();

        // Fetch EC SW tag
        self.ec_sw_tag = self.com.get_ec_sw_tag().ok();

        // Fetch SoC git rev
        self.soc_git_rev = self.llio.soc_gitrev().ok();

        // Fetch SoC DNA (unique device ID)
        self.soc_dna = self.llio.soc_dna().ok();

        // Fetch ADC values
        self.adc_vbus = self.llio.adc_vbus().ok();
        self.adc_temp = self.llio.adc_temperature().ok();
        self.adc_vccint = self.llio.adc_vccint().ok();
        self.adc_vccaux = self.llio.adc_vccaux().ok();

        // Fetch gyroscope (accelerometer) data
        if let Ok((x, y, z, _id)) = self.com.gyro_read_blocking() {
            self.gyro = Some((x, y, z));
        }
    }

    pub fn draw(&self, gam: &Gam, gid: gam::Gid, screensize: Point) {
        let content_x = 15isize;
        let content_width = screensize.x - 30;
        let mut y = 45isize;
        let line_height = 18isize;

        // BATTERY section
        ui::draw_section_header(gam, gid, content_x, y, content_width, "BATTERY");
        y += line_height + 4;

        if let Some(stats) = &self.batt_stats {
            let voltage_str = format!("{}.{:02} V", stats.voltage / 1000, (stats.voltage % 1000) / 10);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "Voltage:", &voltage_str);
            y += line_height;

            let soc_str = format!("{}%", stats.soc);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 60, "Charge:", &soc_str);
            // Draw progress bar next to charge
            ui::draw_progress_bar(gam, gid, content_x + 170, y + 2, 120, 12, stats.soc);
            y += line_height;

            let current_str = if stats.current >= 0 {
                format!("+{} mA", stats.current)
            } else {
                format!("{} mA", stats.current)
            };
            let charging_status = if self.charging.unwrap_or(false) {
                " (charging)"
            } else if stats.current < 0 {
                " (discharging)"
            } else {
                ""
            };
            ui::draw_label_value(
                gam,
                gid,
                content_x + 10,
                y,
                100,
                200,
                "Current:",
                &format!("{}{}", current_str, charging_status),
            );
            y += line_height;

            let cap_str = format!("{} mAh", stats.remaining_capacity);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "Remaining:", &cap_str);
            y += line_height;
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "Status:", "unavailable");
            y += line_height * 4;
        }

        y += 8;

        // SYSTEM section
        ui::draw_section_header(gam, gid, content_x, y, content_width, "SYSTEM");
        y += line_height + 4;

        if let Some(uptime_ms) = self.ec_uptime {
            let uptime_str = ui::format_duration_hms(uptime_ms);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 200, "EC Uptime:", &uptime_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "EC Uptime:", "N/A");
        }
        y += line_height;

        if let Some((rev, dirty)) = self.ec_git_rev {
            let rev_str = format!("0x{:08x}{}", rev, if dirty { " (dirty)" } else { "" });
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 200, "EC Git:", &rev_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "EC Git:", "N/A");
        }
        y += line_height;

        if let Some(tag) = &self.ec_sw_tag {
            let tag_str = format!("v{}.{}.{}", tag.maj, tag.min, tag.rev);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "EC FW:", &tag_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "EC FW:", "N/A");
        }
        y += line_height;

        if let Some(soc_rev) = &self.soc_git_rev {
            let soc_str = if let Some(commit) = soc_rev.commit {
                format!("v{}.{}.{} ({:08x})", soc_rev.maj, soc_rev.min, soc_rev.rev, commit)
            } else {
                format!("v{}.{}.{}", soc_rev.maj, soc_rev.min, soc_rev.rev)
            };
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 200, "SoC:", &soc_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "SoC:", "N/A");
        }
        y += line_height;

        if let Some(dna) = self.soc_dna {
            let dna_str = format!("{:016x}", dna);
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 200, "DNA:", &dna_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 100, 150, "DNA:", "N/A");
        }
        y += line_height;

        y += 4;

        // ADC section
        ui::draw_section_header(gam, gid, content_x, y, content_width, "ADC / SENSORS");
        y += line_height + 2;

        // VBUS and Temperature on same conceptual row but use two lines for clarity
        if let Some(vbus) = self.adc_vbus {
            let vbus_mv = (vbus as u32 * 503) / 100;
            let vbus_str = format!("{}.{:02}V", vbus_mv / 1000, (vbus_mv % 1000) / 10);
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 80, "VBUS:", &vbus_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 80, "VBUS:", "N/A");
        }

        if let Some(temp_raw) = self.adc_temp {
            let temp_kelvin = (temp_raw as i32 * 504) / 4096;
            let temp_c = temp_kelvin - 273;
            let temp_str = format!("{}C", temp_c);
            ui::draw_label_value(gam, gid, content_x + 170, y, 80, 60, "Temp:", &temp_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 170, y, 80, 60, "Temp:", "N/A");
        }
        y += line_height;

        // VccInt and VccAux
        if let Some(vccint) = self.adc_vccint {
            // VccInt: raw * 3.0 / 4096
            let mv = (vccint as u32 * 3000) / 4096;
            let v_str = format!("{}.{:02}V", mv / 1000, (mv % 1000) / 10);
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 80, "VccInt:", &v_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 80, "VccInt:", "N/A");
        }

        if let Some(vccaux) = self.adc_vccaux {
            let mv = (vccaux as u32 * 3000) / 4096;
            let v_str = format!("{}.{:02}V", mv / 1000, (mv % 1000) / 10);
            ui::draw_label_value(gam, gid, content_x + 170, y, 80, 80, "VccAux:", &v_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 170, y, 80, 80, "VccAux:", "N/A");
        }
        y += line_height;

        // Gyroscope/Accelerometer
        if let Some((x, y_val, z)) = self.gyro {
            let gyro_str = format!("X:{} Y:{} Z:{}", x as i16, y_val as i16, z as i16);
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 220, "Gyro:", &gyro_str);
        } else {
            ui::draw_label_value(gam, gid, content_x + 10, y, 80, 150, "Gyro:", "N/A");
        }
    }

    pub fn handle_key(&mut self, key: char) {
        match key {
            'r' | 'R' => {
                // Force refresh
                self.refresh();
            }
            _ => {}
        }
    }
}
