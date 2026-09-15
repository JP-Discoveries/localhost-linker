//! Where the popup appears on screen. Pure geometry in physical pixels, so it is testable.

/// Gap between the popup and the screen edge when there is no tray icon to anchor to.
const MARGIN: f64 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Centers the popup on the tray icon and opens it toward the middle of the screen: below a tray
/// at the top (macOS menu bar, most Linux panels), above one at the bottom (Windows taskbar).
pub fn near_tray(icon: Area, popup: (f64, f64), work: Area) -> (f64, f64) {
    let x = icon.x + icon.width / 2.0 - popup.0 / 2.0;
    let tray_on_top = icon.y + icon.height / 2.0 < work.y + work.height / 2.0;
    let y = if tray_on_top { icon.y + icon.height } else { icon.y - popup.1 };
    clamp_into((x, y), popup, work)
}

/// The right-hand corner of the work area, at the bottom or the top.
pub fn corner(popup: (f64, f64), work: Area, bottom: bool) -> (f64, f64) {
    let x = work.x + work.width - popup.0 - MARGIN;
    let y = if bottom { work.y + work.height - popup.1 - MARGIN } else { work.y + MARGIN };
    clamp_into((x, y), popup, work)
}

fn clamp_into((x, y): (f64, f64), popup: (f64, f64), work: Area) -> (f64, f64) {
    let max_x = (work.x + work.width - popup.0).max(work.x);
    let max_y = (work.y + work.height - popup.1).max(work.y);
    (x.clamp(work.x, max_x), y.clamp(work.y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const POPUP: (f64, f64) = (320.0, 480.0);

    fn area(x: f64, y: f64, width: f64, height: f64) -> Area {
        Area { x, y, width, height }
    }

    #[test]
    fn windows_taskbar_opens_above_the_icon() {
        let work = area(0.0, 0.0, 1920.0, 1032.0);
        let icon = area(1700.0, 1040.0, 24.0, 40.0);
        assert_eq!(near_tray(icon, POPUP, work), (1552.0, 552.0));
    }

    #[test]
    fn macos_menu_bar_opens_below_the_icon() {
        let work = area(0.0, 50.0, 2880.0, 1750.0);
        let icon = area(2400.0, 0.0, 44.0, 50.0);
        assert_eq!(near_tray(icon, POPUP, work), (2262.0, 50.0));
    }

    #[test]
    fn icon_near_the_screen_edge_keeps_the_popup_on_screen() {
        let work = area(0.0, 0.0, 1920.0, 1032.0);
        let icon = area(1900.0, 1040.0, 24.0, 40.0);
        assert_eq!(near_tray(icon, POPUP, work).0, 1600.0);
    }

    #[test]
    fn corner_respects_a_monitor_offset() {
        let work = area(-1920.0, 0.0, 1920.0, 1080.0);
        assert_eq!(corner(POPUP, work, true), (-332.0, 588.0));
        assert_eq!(corner(POPUP, work, false), (-332.0, 12.0));
    }
}
