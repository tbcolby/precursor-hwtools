use crate::storage::Settings;
use crate::ui;
use gam::menu::*;
use gam::{Gam, GlyphStyle};
use llio::Llio;

// Note: The llio service exposes gpio_data_direction() to set pin directions,
// but gpio_data_out() and gpio_data_in() are not exposed in the public API.
// This means we can configure directions but cannot directly read/write pin values.
// For pins 2 and 5, we can read voltage via ADC which gives us input capability.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinDirection {
    Input,
    Output,
}

pub struct GpioTab {
    llio: Llio,

    // Per-pin configuration (8 GPIO pins in battery compartment)
    directions: [PinDirection; 8],
    outputs: [bool; 8],    // Desired output values (can't write to HW yet)
    inputs: [bool; 8],     // Inferred from ADC for pins 2,5

    // ADC values (only pins 2 and 5 have ADC capability)
    adc_gpio2: Option<u16>,
    adc_gpio5: Option<u16>,

    // UI state
    selected_pin: usize,
}

impl GpioTab {
    pub fn new(xns: &xous_names::XousNames, settings: &Settings) -> Self {
        let llio = Llio::new(xns);

        // Restore directions from settings
        let mut directions = [PinDirection::Input; 8];
        for i in 0..8 {
            if (settings.gpio_directions >> i) & 1 == 1 {
                directions[i] = PinDirection::Output;
            }
        }

        // Restore output values from settings
        let mut outputs = [false; 8];
        for i in 0..8 {
            outputs[i] = (settings.gpio_outputs >> i) & 1 == 1;
        }

        GpioTab {
            llio,
            directions,
            outputs,
            inputs: [false; 8],
            adc_gpio2: None,
            adc_gpio5: None,
            selected_pin: 0,
        }
    }

    pub fn refresh(&mut self) {
        // Apply current direction configuration to hardware
        let mut dir_mask: u8 = 0;
        for i in 0..8 {
            if self.directions[i] == PinDirection::Output {
                dir_mask |= 1 << i;
            }
        }
        self.llio.gpio_data_direction(dir_mask).ok();

        // Read input values
        // Note: gpio_data_in is not exposed in llio_lib, so we'll use ADC for pins 2/5
        // and mark others as unknown for now

        // Read ADC values for pins 2 and 5
        self.adc_gpio2 = self.llio.adc_gpio2().ok();
        self.adc_gpio5 = self.llio.adc_gpio5().ok();

        // Infer digital input from ADC values (threshold at ~1.5V = ~1200 raw)
        if let Some(adc) = self.adc_gpio2 {
            self.inputs[2] = adc > 1200;
        }
        if let Some(adc) = self.adc_gpio5 {
            self.inputs[5] = adc > 1200;
        }
    }

    pub fn draw(&self, gam: &Gam, gid: gam::Gid, screensize: Point) {
        let content_x = 15isize;
        let content_width = screensize.x - 30;
        let mut y = 45isize;
        let line_height = 20isize;

        ui::draw_section_header(gam, gid, content_x, y, content_width, "GPIO PINS");
        y += line_height + 4;

        // Draw header row
        let col_pin = content_x + 10;

        let mut header_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(col_pin, y),
                Point::new(content_x + content_width, y + 16),
            )),
        );
        header_tv.style = GlyphStyle::Small;
        header_tv.draw_border = false;
        header_tv.clear_area = true;
        use std::fmt::Write;
        write!(header_tv.text, "Pin   Dir    Out   In    ADC").unwrap();
        gam.post_textview(&mut header_tv).ok();
        y += line_height;

        // Draw separator line
        gam.draw_line(
            gid,
            Line::new_with_style(
                Point::new(col_pin, y),
                Point::new(content_x + content_width - 10, y),
                DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1),
            ),
        )
        .ok();
        y += 4;

        // Draw each pin row
        for pin in 0..8 {
            let is_selected = pin == self.selected_pin;
            let has_adc = pin == 2 || pin == 5;

            // Highlight selected row
            if is_selected {
                gam.draw_rectangle(
                    gid,
                    Rectangle::new_with_style(
                        Point::new(col_pin - 5, y),
                        Point::new(content_x + content_width - 5, y + line_height - 2),
                        DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0),
                    ),
                )
                .ok();
            }

            let dir_str = match self.directions[pin] {
                PinDirection::Input => "IN ",
                PinDirection::Output => "OUT",
            };

            let out_str = if self.directions[pin] == PinDirection::Output {
                if self.outputs[pin] { "[1]" } else { "[0]" }
            } else {
                " - "
            };

            let in_str = if self.directions[pin] == PinDirection::Input {
                if has_adc {
                    if self.inputs[pin] { "1" } else { "0" }
                } else {
                    "?"
                }
            } else {
                "-"
            };

            let adc_str = if has_adc {
                match if pin == 2 { self.adc_gpio2 } else { self.adc_gpio5 } {
                    Some(raw) => {
                        // Convert raw ADC to voltage: raw * 3.3 / 4096
                        let mv = (raw as u32 * 3300) / 4096;
                        format!("{}.{:02}V", mv / 1000, (mv % 1000) / 10)
                    }
                    None => "N/A".to_string(),
                }
            } else {
                "-".to_string()
            };

            let marker = if has_adc { "\u{25B8}" } else { " " }; // triangle marker for ADC pins

            let row_text = format!(
                "{}{:>2}   {}   {}    {}   {}",
                marker, pin, dir_str, out_str, in_str, adc_str
            );

            let mut tv = TextView::new(
                gid,
                TextBounds::BoundingBox(Rectangle::new(
                    Point::new(col_pin - 2, y + 2),
                    Point::new(content_x + content_width - 10, y + line_height),
                )),
            );
            tv.style = GlyphStyle::Monospace;
            tv.draw_border = false;
            tv.clear_area = false;
            if is_selected {
                tv.invert = true;
            }
            write!(tv.text, "{}", row_text).unwrap();
            gam.post_textview(&mut tv).ok();

            y += line_height;
        }

        y += 8;

        // Legend and notes
        let mut legend_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(content_x + 10, y),
                Point::new(content_x + content_width, y + 16),
            )),
        );
        legend_tv.style = GlyphStyle::Small;
        legend_tv.draw_border = false;
        legend_tv.clear_area = true;
        write!(legend_tv.text, "\u{25B8} = ADC capable  |  Dir change works").unwrap();
        gam.post_textview(&mut legend_tv).ok();
        y += 16;

        // Note about limitations
        let mut note_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(content_x + 10, y),
                Point::new(content_x + content_width, y + 32),
            )),
        );
        note_tv.style = GlyphStyle::Small;
        note_tv.draw_border = false;
        note_tv.clear_area = true;
        write!(note_tv.text, "Note: Output toggle not yet supported by API").unwrap();
        gam.post_textview(&mut note_tv).ok();
    }

    pub fn handle_key(&mut self, key: char) {
        match key {
            '\u{2191}' => {
                // up arrow
                if self.selected_pin > 0 {
                    self.selected_pin -= 1;
                } else {
                    self.selected_pin = 7;
                }
            }
            '\u{2193}' => {
                // down arrow
                if self.selected_pin < 7 {
                    self.selected_pin += 1;
                } else {
                    self.selected_pin = 0;
                }
            }
            '0'..='7' => {
                // Toggle specific pin output
                let pin = (key as u8 - b'0') as usize;
                if self.directions[pin] == PinDirection::Output {
                    self.outputs[pin] = !self.outputs[pin];
                    // Apply to hardware - would need gpio_dout method
                    log::info!("GPIO{} output toggled to {}", pin, self.outputs[pin]);
                } else {
                    // Select and switch to output mode
                    self.selected_pin = pin;
                    self.directions[pin] = PinDirection::Output;
                    log::info!("GPIO{} switched to output mode", pin);
                }
            }
            'i' | 'I' => {
                // Set selected pin to input
                self.directions[self.selected_pin] = PinDirection::Input;
                log::info!("GPIO{} set to input mode", self.selected_pin);
            }
            'o' | 'O' => {
                // Set selected pin to output
                self.directions[self.selected_pin] = PinDirection::Output;
                log::info!("GPIO{} set to output mode", self.selected_pin);
            }
            ' ' => {
                // Toggle selected pin if output
                if self.directions[self.selected_pin] == PinDirection::Output {
                    self.outputs[self.selected_pin] = !self.outputs[self.selected_pin];
                    log::info!(
                        "GPIO{} toggled to {}",
                        self.selected_pin,
                        self.outputs[self.selected_pin]
                    );
                }
            }
            'a' | 'A' => {
                // Show ADC value for selected pin (only 2 and 5)
                if self.selected_pin == 2 {
                    if let Some(adc) = self.adc_gpio2 {
                        log::info!("GPIO2 ADC: {} raw", adc);
                    }
                } else if self.selected_pin == 5 {
                    if let Some(adc) = self.adc_gpio5 {
                        log::info!("GPIO5 ADC: {} raw", adc);
                    }
                } else {
                    log::info!("GPIO{} does not have ADC capability", self.selected_pin);
                }
            }
            _ => {}
        }
    }

    pub fn save_to_settings(&self, settings: &mut Settings) {
        let mut dir_mask: u8 = 0;
        let mut out_mask: u8 = 0;
        for i in 0..8 {
            if self.directions[i] == PinDirection::Output {
                dir_mask |= 1 << i;
            }
            if self.outputs[i] {
                out_mask |= 1 << i;
            }
        }
        settings.gpio_directions = dir_mask;
        settings.gpio_outputs = out_mask;
    }
}
