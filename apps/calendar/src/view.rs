//! The screen: a title, today's and tomorrow's events in a crown-scrolled list, a status line at the bottom.
//! Full redraw on every change (a change is rare: an answer, a crown step, a new minute).

use crate::model::{Buf, Item, MAX, Model, Status, age, hhmm, label};
use cmf_wp2_ui::color::{BLACK, Rgb, dim565};
use cmf_wp2_ui::draw::rrect;
use cmf_wp2_ui::geom::text_box;
use cmf_wp2_ui::text::{centre, fitted};
use cmf_wp2_ui::widget::List;
use cmf_wp2_ui::{C, CHAR, ONE, SIZE, Surface};
use core::fmt::Write;

const WHITE: u16 = 0xFFFF;
const GREY: u16 = 0x8C71;
const DIM: u16 = 0x52AA;
const ACCENT: u16 = 0xFD20;
const TITLE_Y: i32 = 44;
const FOOTER_Y: i32 = 372;
const TEXT_X: i32 = 20;

pub fn list() -> List {
    List::new(84, 56, 5, 10)
}

pub fn frame(s: &mut impl Surface, m: &Model, wall: i32, list: &mut List) {
    s.rect(0, 0, SIZE, SIZE, 0);
    centre(s, C, TITLE_Y, "Calendar", ACCENT);
    let message: Option<(&str, &str)> = match m.status {
        Status::Ask => Some(("Allow on your phone", "in FMC Companion")),
        Status::Denied => Some(("No calendar access", "Allow it in FMC Companion")),
        _ if m.fetched.is_none() => Some(match (m.status, m.silent) {
            (Status::Unavailable, _) => ("Phone can't answer", "Trying again later"),
            (_, true) => ("Phone not answering", "Open FMC Companion"),
            _ => ("Asking the phone...", ""),
        }),
        _ => None,
    };
    if let Some((a, b)) = message {
        list.set_len(0);
        centre(s, C, 210, a, WHITE);
        centre(s, C, 240, b, GREY);
        return;
    }

    let mut items = [Item::Day(0); 2 * MAX];
    let n = m.items(wall, &mut items);
    list.set_len(n);
    if n == 0 {
        centre(s, C, 210, "No more events", WHITE);
        centre(s, C, 240, "today or tomorrow", GREY);
    }
    // one left edge for every row: the narrowest visible row's
    let left = (0..list.rows).map(|r| list.row(r).x0).max().unwrap_or(0);
    list.draw(s, 0, |s, i, row| match items[i] {
        Item::Day(d) => {
            s.text(left, row.y + 30, if d == 0 { "Today" } else { "Tomorrow" }, GREY);
        }
        Item::Event(e) => {
            let ev = &m.events[e];
            let colour = if ev.color == 0 { GREY } else { ev.color };
            let colour = if ev.tentative() { dim565(colour, ONE / 2) } else { colour };
            rrect(s, left, row.y + 10, 6, 36, 3, Rgb::from565(colour), BLACK);
            let x = left + TEXT_X;
            let mut when = Buf::<16>::new();
            if ev.all_day() {
                let _ = write!(when, "All day");
            } else {
                hhmm(&mut when, ev.start as i32);
                let _ = write!(when, "-");
                hhmm(&mut when, ev.start as i32 + ev.duration as i32);
            }
            s.text(x, row.y + 6, when.as_str(), if ev.tentative() { GREY } else { WHITE });
            let hidden = ev.title().is_empty();
            fitted(s, x, row.y + 30, label(ev), row.cells(x), if hidden { DIM } else { 0xCE59 });
        }
    });

    let mut line = Buf::<40>::new();
    let fetched = m.fetched.unwrap_or(wall);
    let stale = m.silent || m.status == Status::Unavailable;
    if stale {
        let _ = write!(line, "{}", if m.silent { "No reply, " } else { "Phone busy, " });
        age(&mut line, wall - fetched);
    } else {
        let _ = write!(line, "Updated ");
        hhmm(&mut line, fetched);
    }
    let (_, cells) = text_box(FOOTER_Y, 6);
    let w = line.as_str().chars().count().min(cells) as i32 * CHAR;
    fitted(s, C - w / 2, FOOTER_Y, line.as_str(), cells, if stale { ACCENT } else { GREY });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmf_wp2_ui::Fb;

    #[test]
    fn every_state_draws_inside_the_circle() {
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        let mut m = Model::new();
        let mut l = list();
        frame(&mut fb, &m, 0, &mut l);
        m.status = Status::Ask;
        frame(&mut fb, &m, 0, &mut l);
        // a lit pixel outside the round panel means text runs off the screen
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (dx, dy) = (x - C, y - C);
                if dx * dx + dy * dy > (C - 2) * (C - 2) {
                    assert_eq!(fb.at(x, y), 0, "{x},{y}");
                }
            }
        }
        // every footer variant fits its line
        let (_, cells) = text_box(FOOTER_Y, 6);
        assert!("No reply, 59 min ago".len() <= cells && "Phone busy, 23 h ago".len() <= cells, "{cells}");
    }
}
