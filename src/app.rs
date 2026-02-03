use crate::gpio_tab::GpioTab;
use crate::storage::Settings;
use crate::system_tab::SystemTab;
use crate::uart_tab::UartTab;
use crate::ui;
use crate::APP_NAME;
use crate::AppOp;
use gam::menu::*;
use gam::{Gam, GlyphStyle, UxRegistration};
use num_traits::ToPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    System = 0,
    Gpio = 1,
    Uart = 2,
}

impl AppState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => AppState::System,
            1 => AppState::Gpio,
            2 => AppState::Uart,
            _ => AppState::System,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> Self {
        match self {
            AppState::System => AppState::Gpio,
            AppState::Gpio => AppState::Uart,
            AppState::Uart => AppState::System,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            AppState::System => AppState::Uart,
            AppState::Gpio => AppState::System,
            AppState::Uart => AppState::Gpio,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AppState::System => "System",
            AppState::Gpio => "GPIO",
            AppState::Uart => "UART",
        }
    }
}

pub struct HwToolsApp {
    gam: Gam,
    gid: gam::Gid,
    screensize: Point,
    _token: [u32; 4],

    state: AppState,
    settings: Settings,

    system_tab: SystemTab,
    gpio_tab: GpioTab,
    uart_tab: UartTab,
}

impl HwToolsApp {
    pub fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = Gam::new(xns).expect("can't connect to GAM");

        let token = gam
            .register_ux(UxRegistration {
                app_name: String::from(APP_NAME),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: AppOp::Redraw.to_u32().unwrap(),
                gotinput_id: None,
                audioframe_id: None,
                rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
                focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
            })
            .expect("couldn't register UX")
            .unwrap();

        let gid = gam.request_content_canvas(token).expect("couldn't get canvas");
        let screensize = gam.get_canvas_bounds(gid).expect("couldn't get dimensions");

        let settings = Settings::load();
        let state = AppState::from_u8(settings.last_tab);

        let system_tab = SystemTab::new(xns);
        let gpio_tab = GpioTab::new(xns, &settings);
        let uart_tab = UartTab::new(xns, &settings);

        HwToolsApp {
            gam,
            gid,
            screensize,
            _token: token,
            state,
            settings,
            system_tab,
            gpio_tab,
            uart_tab,
        }
    }

    pub fn refresh_data(&mut self) {
        match self.state {
            AppState::System => self.system_tab.refresh(),
            AppState::Gpio => self.gpio_tab.refresh(),
            AppState::Uart => self.uart_tab.refresh(),
        }
    }

    pub fn full_redraw(&mut self) {
        ui::clear_screen(&self.gam, self.gid, self.screensize);
        self.draw_tabs();
        self.draw_content();
        self.draw_help();
        self.gam.redraw().unwrap();
    }

    pub fn redraw(&mut self) {
        self.draw_content();
        self.gam.redraw().unwrap();
    }

    fn draw_tabs(&self) {
        let tab_y = 5isize;
        let tab_height = 22isize;
        let tab_width = 100isize;
        let tab_spacing = 8isize;

        for (i, tab_state) in [AppState::System, AppState::Gpio, AppState::Uart]
            .iter()
            .enumerate()
        {
            let x = 10 + (i as isize) * (tab_width + tab_spacing);
            let is_selected = *tab_state == self.state;

            // Draw tab background
            let style = if is_selected {
                DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1)
            } else {
                DrawStyle::new(PixelColor::Light, PixelColor::Dark, 1)
            };

            self.gam
                .draw_rounded_rectangle(
                    self.gid,
                    RoundedRectangle::new(
                        Rectangle::new_with_style(
                            Point::new(x, tab_y),
                            Point::new(x + tab_width, tab_y + tab_height),
                            style,
                        ),
                        4,
                    ),
                )
                .ok();

            // Draw tab label
            let label = format!("{}.{}", i + 1, tab_state.label());
            let mut tv = TextView::new(
                self.gid,
                TextBounds::BoundingBox(Rectangle::new(
                    Point::new(x + 4, tab_y + 3),
                    Point::new(x + tab_width - 4, tab_y + tab_height - 2),
                )),
            );
            tv.style = GlyphStyle::Small;
            tv.draw_border = false;
            tv.clear_area = false;
            if is_selected {
                tv.invert = true;
            }
            use std::fmt::Write;
            write!(tv.text, "{}", label).unwrap();
            self.gam.post_textview(&mut tv).ok();
        }
    }

    fn draw_content(&self) {
        let content_top = 35isize;
        let content_bottom = self.screensize.y - 35;

        // Clear content area
        self.gam
            .draw_rectangle(
                self.gid,
                Rectangle::new_with_style(
                    Point::new(5, content_top),
                    Point::new(self.screensize.x - 5, content_bottom),
                    DrawStyle::new(PixelColor::Light, PixelColor::Light, 0),
                ),
            )
            .ok();

        match self.state {
            AppState::System => self.system_tab.draw(&self.gam, self.gid, self.screensize),
            AppState::Gpio => self.gpio_tab.draw(&self.gam, self.gid, self.screensize),
            AppState::Uart => self.uart_tab.draw(&self.gam, self.gid, self.screensize),
        }
    }

    fn draw_help(&self) {
        let help_y = self.screensize.y - 30;

        // Clear help area
        self.gam
            .draw_rectangle(
                self.gid,
                Rectangle::new_with_style(
                    Point::new(5, help_y),
                    Point::new(self.screensize.x - 5, self.screensize.y - 5),
                    DrawStyle::new(PixelColor::Light, PixelColor::Light, 0),
                ),
            )
            .ok();

        let help_text = match self.state {
            AppState::System => "F2:Refresh  1/2/3:Tab  q:Quit",
            AppState::Gpio => "0-7:Toggle  i/o:Dir  \u{2191}\u{2193}:Select  q:Quit",
            AppState::Uart => "c:Clear  p:Pause  m:Mux  q:Quit",
        };

        let mut tv = TextView::new(
            self.gid,
            TextBounds::BoundingBox(Rectangle::new(
                Point::new(10, help_y + 2),
                Point::new(self.screensize.x - 10, self.screensize.y - 5),
            )),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.clear_area = false;
        use std::fmt::Write;
        write!(tv.text, "{}", help_text).unwrap();
        self.gam.post_textview(&mut tv).ok();
    }

    pub fn handle_key(&mut self, key: char) -> bool {
        // Global keys
        match key {
            'q' | 'Q' => return true, // quit
            '1' => {
                self.switch_tab(AppState::System);
                return false;
            }
            '2' => {
                self.switch_tab(AppState::Gpio);
                return false;
            }
            '3' => {
                self.switch_tab(AppState::Uart);
                return false;
            }
            '\u{2192}' => {
                // right arrow
                self.switch_tab(self.state.next());
                return false;
            }
            '\u{2190}' => {
                // left arrow
                self.switch_tab(self.state.prev());
                return false;
            }
            _ => {}
        }

        // Tab-specific keys
        match self.state {
            AppState::System => self.system_tab.handle_key(key),
            AppState::Gpio => self.gpio_tab.handle_key(key),
            AppState::Uart => self.uart_tab.handle_key(key),
        }

        false
    }

    fn switch_tab(&mut self, new_state: AppState) {
        if new_state != self.state {
            self.state = new_state;
            self.settings.last_tab = new_state.to_u8();
            self.refresh_data();
            self.full_redraw();
        }
    }

    pub fn save_settings(&mut self) {
        self.gpio_tab.save_to_settings(&mut self.settings);
        self.uart_tab.save_to_settings(&mut self.settings);
        self.settings.save();
    }
}
