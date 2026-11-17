#![cfg_attr(any(target_os = "none", wp2_sim), no_main, cfg_attr(target_os = "none", no_std))]

#[cfg_attr(not(any(target_os = "none", wp2_sim)), allow(dead_code))]
mod model;
#[cfg_attr(not(any(target_os = "none", wp2_sim)), allow(dead_code))]
mod rpc;
#[cfg_attr(not(any(target_os = "none", wp2_sim)), allow(dead_code))]
mod view;

#[cfg(not(any(target_os = "none", wp2_sim)))]
fn main() {}

#[cfg(any(target_os = "none", wp2_sim))]
mod device {
    use crate::model::{BLOB_MAX, Buf, Got, Model, wall_min};
    use crate::view;
    use cmf_wp2_app::{Api, BUTTON, CROWN, MSG_MAX, PAUSE, RESUME};
    use core::fmt::Write;
    use core::ptr::addr_of_mut;

    cmf_wp2_app::entry!(main);
    cmf_wp2_app::app_icon!();
    cmf_wp2_app::app_features!(cmf_wp2_app::WIDGET | cmf_wp2_app::PAGE);

    // ponytail: ~2.3 KB with the pending answer, kept off the 8 KiB stack
    static mut MODEL: Model = Model::new();
    static mut BLOB: [u8; BLOB_MAX] = [0; BLOB_MAX];

    fn wall(api: &Api) -> Option<i32> {
        let t = api.time()?;
        Some(wall_min(t.year as i32, t.month as u32, t.day as u32, t.hour as u32, t.minute as u32))
    }

    fn to_next_minute(api: &Api) -> u32 {
        api.time().map_or(60_000, |t| 60_000 - (t.second as u32 * 1000 + t.millis as u32).min(59_999))
    }

    // true on RESUME, false on BUTTON
    fn paused(api: &Api) -> bool {
        loop {
            match api.event(u32::MAX).map(|e| e.kind) {
                Some(RESUME) => return true,
                Some(BUTTON) => return false,
                _ => {}
            }
        }
    }

    fn main(api: &'static Api) -> i32 {
        let m = unsafe { &mut *addr_of_mut!(MODEL) };
        let blob = unsafe { &mut *addr_of_mut!(BLOB) };
        let n = api.load(blob);
        if n > 0 {
            m.load(&blob[..n as usize]);
        }
        m.ask_now(api.now());
        let mut screen = api;
        let mut list = view::list();
        let mut drawn = None;
        let mut card = Buf::<64>::new();
        let mut msg = [0u8; MSG_MAX];
        loop {
            let now = api.now();
            let w = wall(api).unwrap_or(m.fetched.unwrap_or(0));
            while let Some(data) = api.recv(&mut msg) {
                if m.reply(data, now, w) == Got::Events {
                    let len = m.save(blob);
                    api.save(&blob[..len]); // on change: the app can end without notice
                }
            }
            if let Some(req) = m.poll(now) {
                api.send(&req); // -107 with no phone: the retry timer covers it
            }
            if drawn != Some((m.rev, w)) {
                view::frame(&mut screen, m, w, &mut list);
                api.show();
                drawn = Some((m.rev, w));
                let (mut title, mut value) = (Buf::<24>::new(), Buf::<40>::new());
                m.widget(w, &mut title, &mut value);
                let mut both = Buf::<64>::new();
                let _ = write!(both, "{}\n{}", title.as_str(), value.as_str());
                if both.as_str() != card.as_str() {
                    api.widget(title.as_str(), value.as_str()); // NAND: only when the text changes
                    card = both;
                }
            }
            let wait = m.wait(api.now()).min(to_next_minute(api));
            let Some(e) = api.event(wait) else { continue };
            match e.kind {
                BUTTON => return 0,
                PAUSE => {
                    if !paused(api) {
                        return 0;
                    }
                    m.ask_now(api.now());
                    drawn = None;
                }
                CROWN => {
                    if list.crown(e.delta) {
                        drawn = None;
                    }
                }
                _ => {} // MESSAGE: drained at the top
            }
        }
    }
}
