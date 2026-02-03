use crate::storage::Settings;
use crate::ui;
use gam::menu::*;
use gam::{Gam, GlyphStyle};
use llio::Llio;
use llio::UartType;
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 100;
const VISIBLE_LINES: usize = 15;

#[derive(Debug, Clone)]
struct LogLine {
    timestamp_ms: u64,
    content: String,
}

// Helper to convert UartType to u8 for storage
fn uart_mux_to_u8(mux: &UartType) -> u8 {
    match mux {
        UartType::Kernel => 0,
        UartType::Log => 1,
        UartType::Application => 2,
        UartType::Invalid => 1,
    }
}

fn u8_to_uart_mux(v: u8) -> UartType {
    match v {
        0 => UartType::Kernel,
        1 => UartType::Log,
        2 => UartType::Application,
        _ => UartType::Log,
    }
}

pub struct UartTab {
    llio: Llio,

    // Log buffer (circular)
    log_lines: VecDeque<LogLine>,

    // State - store as u8 to avoid Clone issues
    current_mux_idx: u8,
    paused: bool,
    scroll_offset: usize,

    // TX buffer (for future use)
    tx_buffer: String,
}

impl UartTab {
    pub fn new(xns: &xous_names::XousNames, settings: &Settings) -> Self {
        let llio = Llio::new(xns);

        let current_mux_idx = settings.uart_mux.min(2);

        // Set initial UART mux
        llio.set_uart_mux(u8_to_uart_mux(current_mux_idx)).ok();

        UartTab {
            llio,
            log_lines: VecDeque::with_capacity(MAX_LOG_LINES),
            current_mux_idx,
            paused: false,
            scroll_offset: 0,
            tx_buffer: String::new(),
        }
    }

    pub fn refresh(&mut self) {
        // Note: Actual log capture would require hooking into the logging system
        // or having a way to read UART RX. For now, we'll just show status.

        // In a real implementation, this would:
        // 1. Read from a log ring buffer
        // 2. Parse and timestamp new entries
        // 3. Add to log_lines

        // For demonstration, add a placeholder if buffer is empty
        if self.log_lines.is_empty() && !self.paused {
            self.add_log_line("(UART monitor active - see hardware for actual data)");
        }
    }

    fn add_log_line(&mut self, content: &str) {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let timestamp_ms = tt.elapsed_ms();

        if self.log_lines.len() >= MAX_LOG_LINES {
            self.log_lines.pop_front();
        }

        self.log_lines.push_back(LogLine {
            timestamp_ms,
            content: content.to_string(),
        });
    }

    pub fn draw(&self, gam: &Gam, gid: gam::Gid, screensize: Point) {
        let content_x = 15isize;
        let content_width = screensize.x - 30;
        let mut y = 45isize;
        let line_height = 16isize;

        // UART Monitor header
        ui::draw_section_header(gam, gid, content_x, y, content_width, "UART MONITOR");
        y += 20;

        // Draw mux selector and status
        let mux_names = ["Kernel", "Log", "App"];
        let mux_index = self.current_mux_idx as usize;

        let mut mux_text = String::from("Mux: ");
        for (i, name) in mux_names.iter().enumerate() {
            if i == mux_index {
                mux_text.push('[');
                mux_text.push_str(name);
                mux_text.push(']');
            } else {
                mux_text.push(' ');
                mux_text.push_str(name);
                mux_text.push(' ');
            }
            if i < 2 {
                mux_text.push_str("  ");
            }
        }
        mux_text.push_str("  115200 bd");

        let mut mux_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(content_x + 5, y),
                Point::new(content_x + content_width, y + 16),
            )),
        );
        mux_tv.style = GlyphStyle::Small;
        mux_tv.draw_border = false;
        mux_tv.clear_area = true;
        use std::fmt::Write;
        write!(mux_tv.text, "{}", mux_text).unwrap();
        gam.post_textview(&mut mux_tv).ok();
        y += 20;

        // Status indicator
        let status_text = if self.paused {
            "Status: PAUSED"
        } else {
            "Status: Capturing"
        };
        let mut status_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(content_x + 5, y),
                Point::new(content_x + 200, y + 16),
            )),
        );
        status_tv.style = GlyphStyle::Small;
        status_tv.draw_border = false;
        status_tv.clear_area = true;
        write!(status_tv.text, "{}", status_text).unwrap();
        gam.post_textview(&mut status_tv).ok();
        y += 20;

        // Draw log box outline
        let log_top = y;
        let log_bottom = screensize.y - 80;
        let log_height = log_bottom - log_top;

        gam.draw_rectangle(
            gid,
            Rectangle::new_with_style(
                Point::new(content_x, log_top),
                Point::new(content_x + content_width, log_bottom),
                DrawStyle::new(PixelColor::Light, PixelColor::Dark, 1),
            ),
        )
        .ok();

        // Draw log lines
        let max_visible = ((log_height - 8) / line_height) as usize;
        let visible_lines = max_visible.min(VISIBLE_LINES);

        let total_lines = self.log_lines.len();
        let start_idx = if total_lines > visible_lines + self.scroll_offset {
            total_lines - visible_lines - self.scroll_offset
        } else {
            0
        };

        let mut log_y = log_top + 4;
        for i in start_idx..total_lines.min(start_idx + visible_lines) {
            if let Some(line) = self.log_lines.get(i) {
                let time_secs = line.timestamp_ms / 1000;
                let time_ms = (line.timestamp_ms % 1000) / 10;
                let time_str = format!("[{:02}:{:02}]", time_secs % 60, time_ms);

                // Truncate content to fit
                let max_chars = 35;
                let content = if line.content.len() > max_chars {
                    format!("{}...", &line.content[..max_chars - 3])
                } else {
                    line.content.clone()
                };

                let mut line_tv = TextView::new(
                    gid,
                    TextBounds::BoundingBox(Rectangle::new(
                        Point::new(content_x + 5, log_y),
                        Point::new(content_x + content_width - 5, log_y + line_height),
                    )),
                );
                line_tv.style = GlyphStyle::Small;
                line_tv.draw_border = false;
                line_tv.clear_area = false;
                write!(line_tv.text, "{} {}", time_str, content).unwrap();
                gam.post_textview(&mut line_tv).ok();

                log_y += line_height;
            }
        }

        // Draw scroll indicator if needed
        if total_lines > visible_lines {
            let scroll_pct = if total_lines > visible_lines {
                ((total_lines - visible_lines - self.scroll_offset) * 100) / (total_lines - visible_lines)
            } else {
                100
            };
            let indicator_y = log_top + 5 + ((log_height - 20) as usize * scroll_pct / 100) as isize;

            // Small triangle indicator
            let mut ind_tv = TextView::new(
                gid,
                TextBounds::BoundingBox(Rectangle::new(
                    Point::new(content_x + content_width - 15, indicator_y),
                    Point::new(content_x + content_width - 5, indicator_y + 12),
                )),
            );
            ind_tv.style = GlyphStyle::Small;
            ind_tv.draw_border = false;
            ind_tv.clear_area = false;
            write!(ind_tv.text, "\u{25BC}").unwrap(); // down triangle
            gam.post_textview(&mut ind_tv).ok();
        }

        // TX input field (placeholder)
        y = log_bottom + 5;
        let mut tx_tv = TextView::new(
            gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(content_x, y),
                Point::new(content_x + content_width, y + 20),
            )),
        );
        tx_tv.style = GlyphStyle::Monospace;
        tx_tv.draw_border = true;
        tx_tv.border_width = 1;
        tx_tv.clear_area = true;
        tx_tv.margin = Point::new(4, 2);

        let display_text = if self.tx_buffer.is_empty() {
            "TX: (type to send)".to_string()
        } else {
            format!("TX: {}_", self.tx_buffer)
        };
        write!(tx_tv.text, "{}", display_text).unwrap();
        gam.post_textview(&mut tx_tv).ok();
    }

    pub fn handle_key(&mut self, key: char) {
        match key {
            'c' | 'C' => {
                // Clear log buffer
                self.log_lines.clear();
                self.scroll_offset = 0;
                log::info!("UART log buffer cleared");
            }
            'p' | 'P' => {
                // Toggle pause
                self.paused = !self.paused;
                log::info!("UART capture {}", if self.paused { "paused" } else { "resumed" });
            }
            'm' | 'M' => {
                // Cycle UART mux: 0 -> 1 -> 2 -> 0
                self.current_mux_idx = (self.current_mux_idx + 1) % 3;
                self.llio.set_uart_mux(u8_to_uart_mux(self.current_mux_idx)).ok();
                let mux_name = match self.current_mux_idx {
                    0 => "Kernel",
                    1 => "Log",
                    2 => "Application",
                    _ => "Unknown",
                };
                log::info!("UART mux set to {}", mux_name);
            }
            '\u{2191}' => {
                // up arrow - scroll up
                if self.scroll_offset < self.log_lines.len().saturating_sub(VISIBLE_LINES) {
                    self.scroll_offset += 1;
                }
            }
            '\u{2193}' => {
                // down arrow - scroll down
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            '\r' | '\n' => {
                // Send TX buffer (placeholder - actual send not implemented)
                if !self.tx_buffer.is_empty() {
                    log::info!("TX: {}", self.tx_buffer);
                    self.add_log_line(&format!("TX> {}", self.tx_buffer));
                    self.tx_buffer.clear();
                }
            }
            '\u{0008}' => {
                // Backspace
                self.tx_buffer.pop();
            }
            c if c.is_ascii_graphic() || c == ' ' => {
                // Add to TX buffer (limit length)
                if self.tx_buffer.len() < 64 {
                    self.tx_buffer.push(c);
                }
            }
            _ => {}
        }
    }

    pub fn save_to_settings(&self, settings: &mut Settings) {
        settings.uart_mux = self.current_mux_idx;
    }
}
