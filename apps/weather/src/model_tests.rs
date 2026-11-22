extern crate std;
use super::*;
use std::vec::Vec;

// 2026-11-20 (a Friday) 13:20
const WALL: i32 = 20_777 * 1440 + 13 * 60 + 20;

fn msg(id: u8, status: u8, index: u8, total: u8, record: &[u8]) -> Vec<u8> {
    [&b"R1A"[..], &[id, status, index, total], record].concat()
}

/// The id of the request the model sends now.
fn sent(m: &mut Model, now: u32) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(q) = m.poll(now) {
        out.push(q.to_vec());
    }
    out
}

const NOW: [u8; 7] = [0, 0xDE, 0xFF, 71, 5, 88, 10]; // -3.4°, snow, 5 m/s, 88 %, 10 min old
fn hours(first: u8, n: usize) -> Vec<u8> {
    let mut r = Vec::from([2, first]);
    for i in 0..n {
        r.extend([(i as i8 - 4) as u8, 3, if i >= 6 { 0x80 | 40 } else { 20 }]);
    }
    r
}
fn day(d: i8, min: i8, max: i8) -> [u8; 6] {
    [1, d as u8, min as u8, max as u8, 3, 40]
}

/// A model that asked and got `recs` as its weather, at WALL; the place refused.
fn answered(recs: &[Vec<u8>]) -> Model {
    answered_in(recs, "")
}

fn answered_in(recs: &[Vec<u8>], place: &str) -> Model {
    answered_at(recs, place, WALL)
}

/// The same, with the watch's clock at `wall` when the answer comes.
fn answered_at(recs: &[Vec<u8>], place: &str, wall: i32) -> Model {
    let mut m = Model::new();
    m.ask_now(0);
    let q = sent(&mut m, 0);
    let l = q.iter().find(|q| q[4] == 0x20).expect("asks the place");
    if place.is_empty() {
        m.reply(&msg(l[3], rpc::DENIED, 0, 0, &[]), 50, wall);
    } else {
        m.reply(&msg(l[3], 0, 0, 1, &[&[0u8; 6][..], place.as_bytes()].concat()), 50, wall);
    }
    let weather = q.iter().find(|q| q[4] == 0x30).expect("asks the weather");
    for (i, r) in recs.iter().enumerate() {
        m.reply(&msg(weather[3], 0, i as u8, recs.len() as u8, r), 100, wall);
    }
    m
}

fn week() -> Vec<Vec<u8>> {
    let mut v = Vec::from([NOW.to_vec(), hours(13, 12)]);
    for d in 0..7 {
        v.push(day(d, -5 - d, 1 - d).to_vec());
    }
    v
}

#[test]
fn asks_location_and_weather_with_hours() {
    let mut m = Model::new();
    m.ask_now(0);
    let q = sent(&mut m, 0);
    assert_eq!(q.len(), 2);
    assert!(q.iter().any(|q| q[4] == 0x20 && q.len() == 5), "location");
    assert!(q.iter().any(|q| q[4] == 0x30 && q[5..] == [7, 12]), "7 days, 12 hours");
    assert_eq!(m.status, Status::Waiting);
}

#[test]
fn an_answer_fills_now_hours_and_days() {
    let m = answered(&week());
    assert_eq!(m.status, Status::Ok);
    assert_eq!(m.now, Some(Now { temp: -34, code: 71, wind: 5, humidity: 88 }));
    assert_eq!(m.hours_len(WALL), 12);
    let (hod, is_now, h) = m.hour_at(0, WALL).unwrap();
    assert_eq!((hod, is_now, h.temp, h.night), (13, true, -4, false));
    let (hod, is_now, h) = m.hour_at(11, WALL).unwrap();
    assert_eq!((hod, is_now, h.temp, h.night, h.precip), (0, false, 7, true, 40), "13 + 11 is midnight");
    assert_eq!(m.hour_at(12, WALL), None);
    assert_eq!(m.days_len(WALL), 7);
    assert_eq!(m.day_at(0, WALL).map(|(d, today, x)| (d, today, x.min, x.max)), Some((20_777, true, -5, 1)));
    assert_eq!(m.day_at(6, WALL).map(|(d, today, x)| (d, today, x.min)), Some((20_783, false, -11)));
    assert_eq!(m.today(WALL).map(|d| (d.min, d.max)), Some((-5, 1)));
    assert_eq!(m.range(WALL), (-11, 1));
    assert_eq!(m.stale_min(WALL), 10);
    assert!(!m.no_hours);
}

#[test]
fn records_in_any_order_and_days_sorted() {
    let mut v = week();
    v.reverse();
    let m = answered(&v);
    assert_eq!(m.day_at(0, WALL).map(|(d, ..)| d), Some(20_777));
    assert_eq!(m.day_at(6, WALL).map(|(d, ..)| d), Some(20_783));
    assert!(m.now.is_some());
}

#[test]
fn a_cache_moves_with_the_clock() {
    let m = answered(&week());
    // three hours later: the hours that passed are gone, the labels stay absolute
    let later = WALL + 3 * 60;
    assert_eq!(m.hours_len(later), 9);
    assert_eq!(m.hour_at(0, later).map(|(h, n, x)| (h, n, x.temp)), Some((16, true, -1)));
    assert_eq!(m.stale_min(later), 190);
    // two days later: today is the third day of the answer; the hours are all past
    let later = WALL + 2 * 1440;
    assert_eq!(m.hours_len(later), 0);
    assert_eq!(m.days_len(later), 5);
    assert_eq!(m.day_at(0, later).map(|(d, t, x)| (d, t, x.min)), Some((20_779, true, -7)));
    assert_eq!(m.today(later).map(|d| d.max), Some(-1));
    // a clock set back: nothing before the answer's first hour
    assert_eq!(m.hours_len(WALL - 600), 12);
    // ten days later: no day left
    assert_eq!((m.days_len(WALL + 10 * 1440), m.today(WALL + 10 * 1440)), (0, None));
}

#[test]
fn fewer_hours_than_asked_or_none() {
    let m = answered(&[NOW.to_vec(), hours(22, 5), day(0, 1, 2).to_vec()]);
    assert_eq!(m.hours_len(WALL), 5);
    assert_eq!((0..5).map(|k| m.hour_at(k, WALL).unwrap().0).collect::<Vec<_>>(), [22, 23, 0, 1, 2]);
    assert_eq!(m.hour_at(5, WALL), None);
    // an hourly record without entries, and no hourly record (an iPhone with an old companion)
    let m = answered(&[NOW.to_vec(), hours(13, 0)]);
    assert_eq!((m.hours_len(WALL), m.hour_at(0, WALL), m.no_hours), (0, None, true));
    let m = answered(&[NOW.to_vec(), day(0, 1, 2).to_vec()]);
    assert_eq!((m.hours_len(WALL), m.no_hours, m.status), (0, true, Status::Ok));
}

#[test]
fn an_old_android_companion_is_asked_again_without_hours() {
    let mut m = Model::new();
    m.ask_now(0);
    let q = sent(&mut m, 0);
    let w = q.iter().find(|q| q[4] == 0x30).unwrap();
    assert_eq!(m.reply(&msg(w[3], rpc::BAD_REQUEST, 0, 0, &[]), 10, WALL), Got::Nothing);
    let q = sent(&mut m, 10);
    assert_eq!(q.len(), 1);
    assert_eq!(q[0][4..], [0x30, 7], "days only");
    m.reply(&msg(q[0][3], 0, 0, 2, &NOW), 20, WALL);
    assert_eq!(m.reply(&msg(q[0][3], 0, 1, 2, &day(0, 1, 2)), 20, WALL), Got::Saved);
    assert!(m.no_hours && m.status == Status::Ok);
    // and BAD_REQUEST to that is not asked a third time at once
    m.ask_now(30);
    let q = sent(&mut m, 30);
    let w = q.iter().find(|q| q[4] == 0x30).unwrap();
    assert_eq!(w[5..], [7]);
    assert_eq!(m.reply(&msg(w[3], rpc::BAD_REQUEST, 0, 0, &[]), 40, WALL), Got::Changed);
    assert!(sent(&mut m, 40).iter().all(|q| q[4] != 0x30));
}

#[test]
fn statuses_keep_the_cache() {
    let mut m = answered(&week());
    for (s, want, again) in [
        (rpc::ASK, Status::Ask, ASK_AGAIN_MS),
        (rpc::DENIED, Status::Denied, PERIOD_MS),
        (rpc::UNAVAILABLE, Status::Unavailable, PERIOD_MS),
    ] {
        m.ask_now(1000);
        let q = sent(&mut m, 1000);
        let w = q.iter().find(|q| q[4] == 0x30).unwrap();
        assert_eq!(m.reply(&msg(w[3], s, 0, 0, &[]), 1100, WALL), Got::Changed);
        assert_eq!((m.status, m.wait(1100)), (want, again));
        assert!(m.now.is_some() && m.days_len(WALL) == 7, "the last answer stays");
    }
    // an answer without the current record is no answer
    m.ask_now(2000);
    let q = sent(&mut m, 2000);
    let w = q.iter().find(|q| q[4] == 0x30).unwrap();
    m.reply(&msg(w[3], 0, 0, 1, &day(0, 9, 9)), 2100, WALL);
    assert_eq!(m.status, Status::Unavailable);
    assert_eq!(m.today(WALL).map(|d| d.max), Some(1));
}

#[test]
fn the_place_comes_from_location_and_its_refusal_is_not_an_error() {
    let mut m = Model::new();
    m.ask_now(0);
    let q = sent(&mut m, 0);
    let l = q.iter().find(|q| q[4] == 0x20).unwrap();
    let rec = [&[0x2Cu8, 2, 0x78, 1, 5, 0][..], "Москва".as_bytes()].concat();
    assert_eq!(m.reply(&msg(l[3], 0, 0, 1, &rec), 10, WALL), Got::Saved);
    assert_eq!(m.place(), "Москва");
    let mut m = Model::new();
    m.ask_now(0);
    let q = sent(&mut m, 0);
    let l = q.iter().find(|q| q[4] == 0x20).unwrap();
    assert_eq!(m.reply(&msg(l[3], rpc::DENIED, 0, 0, &[]), 10, WALL), Got::Nothing);
    assert_eq!((m.place(), m.status), ("", Status::Waiting));
    // answered once, it is not asked again while the app runs
    m.ask_now(100);
    assert!(sent(&mut m, 100).iter().all(|q| q[4] != 0x20));
}

#[test]
fn save_and_load() {
    let m = answered_in(&week(), "Berlin");
    let mut blob = [0u8; BLOB_MAX];
    let n = m.save(&mut blob);
    assert!(n > 0 && n <= BLOB_MAX);
    let mut back = Model::new();
    assert!(back.load(&blob[..n]));
    assert_eq!(back.place(), "Berlin");
    assert_eq!(back.now, m.now);
    assert_eq!(back.status, Status::Waiting, "a cache is not an answer");
    assert_eq!((back.hours_len(WALL), back.days_len(WALL), back.stale_min(WALL)), (12, 7, 10));
    assert_eq!(back.hour_at(11, WALL), m.hour_at(11, WALL));
    assert_eq!(back.day_at(6, WALL), m.day_at(6, WALL));
    // another version, cut short, empty, garbage counts: ignored, the model stays empty
    let mut other = blob;
    other[1] = b'9';
    for bad in [&other[..n], &blob[..n - 1], &blob[..3], &[][..]] {
        let mut x = Model::new();
        assert!(!x.load(bad));
        assert!(x.now.is_none() && x.days_len(WALL) == 0 && x.hours_len(WALL) == 0 && x.place().is_empty());
    }
    // nothing to save before the first answer
    assert_eq!(Model::new().save(&mut blob), 0);
}

#[test]
fn temperatures() {
    assert_eq!([deg(-34), deg(-35), deg(-4), deg(4), deg(5), deg(0), deg(-5), deg(216)], [-3, -4, 0, 0, 1, 0, -1, 22]);
    // the watch's fonts have no degree sign (it draws as a blank): plain numbers
    assert_eq!(temp_text(0).as_str(), "0");
    assert_eq!(temp_text(-12).as_str(), "-12");
    assert_eq!(temp_text(deg(-4)).as_str(), "0", "never -0");
}

#[test]
fn hour_labels_wrap_at_midnight() {
    assert_eq!([hour_label(22, 0), hour_label(22, 1), hour_label(22, 2), hour_label(22, 3)], [22, 23, 0, 1]);
    assert_eq!(hour_label(23, 23), 22);
}

#[test]
fn weekdays() {
    assert_eq!(weekday(20_777), "Fri");
    assert_eq!(weekday(20_779), "Sun");
    assert_eq!(weekday(20_780), "Mon");
    assert_eq!(wall_min(2026, 11, 20, 13, 20), WALL);
}

#[test]
fn icons_and_names() {
    for (code, i, name) in [
        (0, Icon::Clear, "Clear"), (1, Icon::Partly, "Partly cloudy"), (2, Icon::Partly, "Partly cloudy"),
        (3, Icon::Cloud, "Cloudy"), (45, Icon::Fog, "Fog"), (48, Icon::Fog, "Fog"), (55, Icon::Drizzle, "Drizzle"),
        (63, Icon::Rain, "Rain"), (81, Icon::Rain, "Rain"), (73, Icon::Snow, "Snow"), (86, Icon::Snow, "Snow"),
        (95, Icon::Storm, "Storm"), (99, Icon::Storm, "Storm"), (200, Icon::Cloud, "Cloudy"),
    ] {
        assert_eq!((icon(code), condition(code)), (i, name), "{code}");
    }
}

#[test]
fn bars_stay_in_the_track() {
    // (x, length) of a day's range within the week's, on a track 100 wide, at least 12 long
    assert_eq!(bar(-9, 1, -9, 1, 100, 12), (0, 100));
    assert_eq!(bar(-5, 1, -9, 1, 100, 12), (40, 60));
    assert_eq!(bar(3, 3, 3, 3, 100, 12), (0, 100), "a week of one temperature");
    assert_eq!(bar(1, 1, -9, 1, 100, 12), (88, 12), "a point at the warm end keeps its length inside");
    assert_eq!(bar(-9, -9, -9, 1, 100, 12), (0, 12));
    for (min, max) in [(-128i8, 127i8), (5, -5), (0, 0)] {
        let (x, len) = bar(min, max, -20, 20, 100, 12);
        assert!(x >= 0 && len >= 12 && x + len <= 100, "{min}..{max}: {x}+{len}");
    }
}

#[test]
fn updated_line() {
    assert_eq!(updated(10).map(|l| l.as_str().len()), None);
    assert_eq!(updated(45).map(|_| ()), None);
    assert_eq!(updated(46).unwrap().as_str(), "Updated 46 min ago");
    assert_eq!(updated(190).unwrap().as_str(), "Updated 3 h ago");
    assert_eq!(updated(3 * 1440 + 5).unwrap().as_str(), "Updated 3 d ago");
    assert_eq!(updated(-500).map(|_| ()), None, "a clock set back");
}

#[test]
fn the_card() {
    let m = Model::new();
    let (t, v) = m.card(WALL);
    assert_eq!((t.as_str(), v.as_str()), ("Weather", "Open the app"));
    let m = answered(&week());
    let (t, v) = m.card(WALL);
    assert_eq!((t.as_str(), v.as_str()), ("Weather", "-3 Snow\nH 1 L -5"));
    let m = answered_in(&week(), "Санкт-Петербург и область");
    let (t, v) = m.card(WALL);
    assert!(t.as_str().len() <= 23 && t.as_str().starts_with("Санкт-Петер"), "{}", t.as_str());
    assert!(v.as_str().len() <= 39);
    // no day for today: the first line alone
    let m = answered(&[NOW.to_vec()]);
    assert_eq!(m.card(WALL).1.as_str(), "-3 Snow");
}

#[test]
fn hours_and_days_count_from_the_phones_clock_not_the_watchs() {
    // the phone answered in its 13th hour; the watch got it at 14:02: 13 is past, 14 is now
    let at = 20_777 * 1440 + 14 * 60 + 2;
    let m = answered_at(&week(), "", at);
    assert_eq!(m.hours_len(at), 11);
    assert_eq!(m.hour_at(0, at).map(|(h, n, x)| (h, n, x.temp)), Some((14, true, -3)));
    // the other way: the phone is already in 14, the watch still at 13:59; nothing is past, 14 is not now yet
    let at = 20_777 * 1440 + 13 * 60 + 59;
    let mut v = week();
    v[1] = hours(14, 12);
    let m = answered_at(&v, "", at);
    assert_eq!(m.hours_len(at), 12);
    assert_eq!(m.hour_at(0, at).map(|(h, n, _)| (h, n)), Some((14, false)));
    assert_eq!(m.hour_at(0, at + 1).map(|(h, n, _)| (h, n)), Some((14, true)));
    // midnight: the phone answered at 23:59 of the 20th, the watch got it at 00:01 of the 21st
    let at = 20_778 * 1440 + 1;
    let mut v = week();
    v[1] = hours(23, 12);
    let m = answered_at(&v, "", at);
    assert_eq!(m.day_at(0, at).map(|(d, today, x)| (d, today, x.min)), Some((20_778, true, -6)), "the phone's tomorrow");
    assert_eq!(m.days_len(at), 6);
    assert_eq!(m.hour_at(0, at).map(|(h, n, _)| (h, n)), Some((0, true)));
    // and it survives the cache
    let mut blob = [0u8; BLOB_MAX];
    let n = m.save(&mut blob);
    let mut back = Model::new();
    assert!(back.load(&blob[..n]));
    assert_eq!(back.day_at(0, at), m.day_at(0, at));
    assert_eq!(back.hour_at(0, at), m.hour_at(0, at));
}

// The columns card. The phone pushes the same bytes (fmctools/companion): change neither side alone.

/// The current weather, and hourly entries from `first` o'clock: (temperature, code).
fn forecast(temp: i16, code: u8, first: u8, list: &[(i8, u8)]) -> Vec<Vec<u8>> {
    let t = temp.to_le_bytes();
    let mut h = Vec::from([2, first]);
    for &(t, c) in list {
        h.extend([t as u8, c, 0]);
    }
    Vec::from([Vec::from([0, t[0], t[1], code, 5, 80, 0]), h])
}

/// What stub 9 draws from the record at `wall`: caption, icon, value. Its own parser, with Weather's eight icons.
fn drawn(m: &Model, wall: i32) -> Vec<(std::string::String, Option<u8>, std::string::String)> {
    let Some((rec, n)) = m.columns(wall) else { return Vec::new() };
    let Some(wp2_core::widget::Card::Columns(k)) = wp2_core::widget::card(&rec[..n], 8) else {
        panic!("stub 9 would not draw {:?}", &rec[..n]);
    };
    rec[3..n].chunks_exact(21).take(k as usize).map(|c| (text(&c[..10]), Some(c[10]), text(&c[11..]))).collect()
}

fn text(b: &[u8]) -> std::string::String {
    core::str::from_utf8(b).unwrap().trim_end_matches('\0').into()
}

fn at(day: i32, hour: i32, minute: i32) -> i32 {
    day * 1440 + hour * 60 + minute
}

fn captions(m: &Model, wall: i32) -> Vec<std::string::String> {
    drawn(m, wall).into_iter().map(|c| c.0).collect()
}

#[test]
fn columns_golden_bytes() {
    // wp2apps.install.columns_record([("Now",6,"-3°"),("23:00",2,"-4°"),("00:00",0,"-5°"),("01:00",7,"0°")])
    let golden = concat!(
        "ff0104",
        "4e6f7700000000000000", "06", "2d33c2b0000000000000",
        "32333a30300000000000", "02", "2d34c2b0000000000000",
        "30303a30300000000000", "00", "2d35c2b0000000000000",
        "30313a30300000000000", "07", "30c2b000000000000000",
    );
    let now = at(20_777, 22, 20);
    let m = answered_at(&forecast(-34, 71, 22, &[(9, 1), (-4, 3), (-5, 0), (0, 95), (8, 45)]), "", now);
    let (rec, n) = m.columns(now).expect("a card");
    let hex: std::string::String = rec[..n].iter().map(|b| std::format!("{b:02x}")).collect();
    assert_eq!((n, hex.as_str()), (87, golden));
}

#[test]
fn columns_wrap_at_midnight_and_move_with_the_hour() {
    let m = answered_at(&forecast(10, 0, 21, &[(0, 0); 12]), "", at(20_777, 21, 5));
    assert_eq!(captions(&m, at(20_777, 21, 59)), ["Now", "22:00", "23:00", "00:00"]);
    assert_eq!(captions(&m, at(20_777, 22, 0)), ["Now", "23:00", "00:00", "01:00"]);
    assert_eq!(captions(&m, at(20_778, 0, 0)), ["Now", "01:00", "02:00", "03:00"]);
}

#[test]
fn columns_of_a_cache_skip_the_hours_that_passed() {
    let list: Vec<(i8, u8)> = (0..12).map(|i| (i, 3)).collect();
    let m = answered(&forecast(216, 0, 13, &list));
    let mut blob = [0u8; BLOB_MAX];
    let n = m.save(&mut blob);
    let mut back = Model::new();
    assert!(back.load(&blob[..n]));
    let values = |wall| drawn(&back, wall).into_iter().map(|c| (c.0, c.2)).collect::<Vec<_>>();
    let pairs = |p: &[(&str, &str)]| p.iter().map(|&(a, b)| (a.into(), b.into())).collect::<Vec<_>>();
    // 19:20: the three after 19, not the three after the cache's first entry (13)
    assert_eq!(values(WALL + 6 * 60), pairs(&[("Now", "22°"), ("20:00", "7°"), ("21:00", "8°"), ("22:00", "9°")]));
    // 22:20: two are left, then none; two days later none come round again
    assert_eq!(values(WALL + 9 * 60), pairs(&[("Now", "22°"), ("23:00", "10°"), ("00:00", "11°")]));
    assert_eq!(values(WALL + 11 * 60), pairs(&[("Now", "22°")]));
    assert_eq!(values(WALL + 2 * 1440), pairs(&[("Now", "22°")]));
}

#[test]
fn columns_count_from_the_phones_hour() {
    // the phone answered in its 13th hour, the watch got it at 14:02
    let wall = at(20_777, 14, 2);
    let m = answered_at(&forecast(0, 0, 13, &[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]), "", wall);
    assert_eq!(captions(&m, wall), ["Now", "15:00", "16:00", "17:00"]);
    assert_eq!(drawn(&m, wall)[1].2, "3°");
    // the phone is in 14 already, the watch at 13:59: 14 is the next hour
    let wall = at(20_777, 13, 59);
    let m = answered_at(&forecast(0, 0, 14, &[(4, 0), (5, 0), (6, 0), (7, 0)]), "", wall);
    assert_eq!(captions(&m, wall), ["Now", "14:00", "15:00", "16:00"]);
    // a clock set back: only those of the next three hours that are known
    assert_eq!(captions(&m, wall - 60), ["Now", "14:00", "15:00"]);
    assert_eq!(captions(&m, wall - 5 * 60), ["Now"]);
}

#[test]
fn fewer_hours_give_fewer_columns_and_no_weather_none() {
    for (known, want) in [(0, 1), (1, 1), (2, 2), (3, 3), (4, 4), (12, 4)] {
        let m = answered(&forecast(0, 0, 13, &std::vec![(0, 0); known]));
        assert_eq!(drawn(&m, WALL).len(), want, "{known} hours known");
        assert_eq!(m.columns(WALL).map(|r| r.1), Some(3 + 21 * want));
    }
    assert_eq!(drawn(&answered(&[NOW.to_vec()]), WALL).len(), 1, "no hourly record");
    assert!(Model::new().columns(WALL).is_none(), "no current weather");
}

#[test]
fn column_values_round_away_from_zero() {
    for (t10, want) in [
        (-35, "-4°"), (-34, "-3°"), (-5, "-1°"), (-4, "0°"), (0, "0°"), (4, "0°"), (5, "1°"), (25, "3°"),
        (216, "22°"), (i16::MAX, "3277°"), (i16::MIN, "-3277°"),
    ] {
        assert_eq!(drawn(&answered(&forecast(t10, 0, 13, &[])), WALL)[0].2, want, "{t10}");
    }
}

#[test]
fn column_icons_follow_the_contract_for_every_code() {
    for code in 0..=255u8 {
        let want = match code {
            0 => 0,
            1 | 2 => 1,
            45 | 48 => 3,
            51..=57 => 4,
            61..=67 | 80..=82 => 5,
            71..=77 | 85 | 86 => 6,
            95..=99 => 7,
            _ => 2,
        };
        let m = answered(&forecast(0, code, 13, &[(0, 0), (0, code)]));
        let got: Vec<_> = drawn(&m, WALL).into_iter().map(|c| c.1).collect();
        assert_eq!(got, [Some(want), Some(want)], "WMO {code}, Now and an hour");
    }
}

#[test]
fn widget_icon_files_are_in_the_contracts_order() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("widget-icons");
    let mut names: Vec<std::string::String> = std::fs::read_dir(dir)
        .expect("widget-icons/")
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".svg"))
        .collect();
    names.sort();
    let want = ["clear", "partly", "cloud", "fog", "drizzle", "rain", "snow", "storm"];
    let want: Vec<_> = want.iter().enumerate().map(|(i, n)| std::format!("{i:02}-{n}.svg")).collect();
    assert_eq!(names, want, "the package numbers the icons in file name order");
}

/// The writes of one update: (is columns, the text card's value), the watch answering `rc` to columns.
fn writes(w: &mut Widget, m: &Model, wall: i32, rc: i32) -> Vec<(bool, std::string::String)> {
    let mut out = Vec::new();
    w.update(m, wall, |card| match card {
        Card::Columns(..) => {
            out.push((true, "".into()));
            rc
        }
        Card::Text(_, v) => {
            out.push((false, v.as_str().into()));
            0
        }
    });
    out
}

#[test]
fn the_card_is_written_on_a_changed_record_only() {
    let mut w = Widget::new();
    let wall = at(20_777, 13, 20);
    assert_eq!(writes(&mut w, &Model::new(), wall, 0), [(false, "Open the app".into())], "no weather: the text card");
    let m = answered(&week());
    assert_eq!(writes(&mut w, &m, wall, 0), [(true, "".into())]);
    for minute in [20, 21, 59] {
        assert_eq!(writes(&mut w, &m, at(20_777, 13, minute), 0), [], "the same record");
    }
    assert_eq!(writes(&mut w, &m, at(20_777, 14, 0), 0), [(true, "".into())], "the next hour");
}

#[test]
fn a_watch_without_columns_gets_the_text_card_and_is_not_asked_again() {
    let mut w = Widget::new();
    let m = answered(&week());
    let text = (false, "-3 Snow\nH 1 L -5".into());
    assert_eq!(writes(&mut w, &m, WALL, -22), [(true, "".into()), text], "refused: the text card at once");
    assert_eq!(writes(&mut w, &m, WALL, -22), [], "the same text");
    assert_eq!(writes(&mut w, &m, WALL + 60, -22), [], "the next hour, the same text: no second try");
    let warmer = answered(&forecast(50, 0, 13, &[]));
    assert_eq!(writes(&mut w, &warmer, WALL + 60, -22), [(false, "5 Clear".into())], "new text, still no second try");
}

#[test]
fn a_failed_write_is_written_again_and_is_no_reason_for_text() {
    let mut w = Widget::new();
    let m = answered(&week());
    assert_eq!(writes(&mut w, &m, WALL, -5), [(true, "".into())]);
    assert_eq!(writes(&mut w, &m, WALL, 0), [(true, "".into())]);
    assert_eq!(writes(&mut w, &m, WALL, 0), []);
}
