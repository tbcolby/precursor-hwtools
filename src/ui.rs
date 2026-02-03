use gam::menu::*;
use gam::{Gam, GlyphStyle};

pub fn clear_screen(gam: &Gam, gid: gam::Gid, screensize: Point) {
    gam.draw_rectangle(
        gid,
        Rectangle::new_with_style(
            Point::new(0, 0),
            screensize,
            DrawStyle {
                fill_color: Some(PixelColor::Light),
                stroke_color: None,
                stroke_width: 0,
            },
        ),
    )
    .expect("can't clear screen");
}

pub fn draw_section_header(gam: &Gam, gid: gam::Gid, x: isize, y: isize, width: isize, text: &str) {
    let mut tv = TextView::new(
        gid,
        TextBounds::BoundingBox(Rectangle::new(
            Point::new(x, y),
            Point::new(x + width, y + 18),
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.draw_border = false;
    tv.clear_area = true;
    use std::fmt::Write;
    write!(tv.text, "{}", text).unwrap();
    gam.post_textview(&mut tv).ok();
}

pub fn draw_label_value(
    gam: &Gam,
    gid: gam::Gid,
    x: isize,
    y: isize,
    label_width: isize,
    value_width: isize,
    label: &str,
    value: &str,
) {
    // Draw label
    let mut tv = TextView::new(
        gid,
        TextBounds::BoundingBox(Rectangle::new(
            Point::new(x, y),
            Point::new(x + label_width, y + 16),
        )),
    );
    tv.style = GlyphStyle::Regular;
    tv.draw_border = false;
    tv.clear_area = true;
    use std::fmt::Write;
    write!(tv.text, "{}", label).unwrap();
    gam.post_textview(&mut tv).ok();

    // Draw value
    let mut tv = TextView::new(
        gid,
        TextBounds::BoundingBox(Rectangle::new(
            Point::new(x + label_width, y),
            Point::new(x + label_width + value_width, y + 16),
        )),
    );
    tv.style = GlyphStyle::Monospace;
    tv.draw_border = false;
    tv.clear_area = true;
    write!(tv.text, "{}", value).unwrap();
    gam.post_textview(&mut tv).ok();
}

pub fn draw_progress_bar(
    gam: &Gam,
    gid: gam::Gid,
    x: isize,
    y: isize,
    width: isize,
    height: isize,
    percent: u8,
) {
    // Draw outline
    gam.draw_rectangle(
        gid,
        Rectangle::new_with_style(
            Point::new(x, y),
            Point::new(x + width, y + height),
            DrawStyle::new(PixelColor::Light, PixelColor::Dark, 1),
        ),
    )
    .ok();

    // Draw fill
    let fill_width = ((width - 4) as u32 * percent as u32 / 100) as isize;
    if fill_width > 0 {
        gam.draw_rectangle(
            gid,
            Rectangle::new_with_style(
                Point::new(x + 2, y + 2),
                Point::new(x + 2 + fill_width, y + height - 2),
                DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0),
            ),
        )
        .ok();
    }
}

pub fn format_duration_hms(total_ms: u64) -> String {
    let total_secs = total_ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
