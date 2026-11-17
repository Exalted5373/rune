use super::*;
use crate::el::{ScrollSpec, col, el};
use std::time::Instant;

#[derive(Default)]
struct Groups {
    gone: bool,
    hide_a: bool,
}

/// Two groups of four rows. `gone` removes the second row of each, the one in `a` with a dot
/// in it; `hide_a` hides group `a`.
fn groups(s: &Groups, _: &mut Cx<Groups>) -> El {
    let group = |g: &str, hidden: bool| {
        let rows = (0..4).filter(|&i| !(s.gone && i == 1)).map(|i| {
            let row = col().key(format!("{g}{i}")).w_full().h(20.0).fill(Color::WHITE).transition(Transition::opacity());
            if g == "a" { row.child(el().key(format!("{g}{i}-dot")).size(8.0, 8.0).fill(Color::BLACK)) } else { row }
        });
        col().key(g).w_full().opacity(if hidden { 0.0 } else { 1.0 }).children(rows)
    };
    col().w_full().children([group("a", s.hide_a), group("b", false)])
}

/// Whether the node keyed `key` is exiting; `None` once it's gone.
fn exiting<S: 'static>(rt: &Runtime<S>, key: &str) -> Option<bool> {
    let id = rt.nodes.iter().find(|n| n.named(key))?.id;
    Some(matches!(rt.scene.phase(id), Some(PresencePhase::Exiting { .. })))
}

#[test]
fn an_exit_ends_once_nobody_sees_it_and_one_in_sight_plays_on() {
    let mut rt = Runtime::new(Groups::default(), groups, (300.0, 300.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt.frame(1.0 / 120.0);
    rt.update(|s, tx| { s.gone = true; tx.animated(); });
    rt.frame(1.0 / 120.0);
    rt.frame(1.0 / 120.0);
    assert_eq!(exiting(&rt, "a1"), Some(true), "a row in sight plays its exit");
    assert_eq!(exiting(&rt, "b1"), Some(true), "so does the other");
    rt.update(|s, tx| { s.hide_a = true; tx.instant(); });
    rt.frame(1.0 / 120.0);
    rt.frame(1.0 / 120.0);
    assert_eq!(exiting(&rt, "a1"), None, "an exit under a group that hid ends at once");
    assert_eq!(exiting(&rt, "a1-dot"), None, "with what it holds");
    assert_eq!(exiting(&rt, "b1"), Some(true), "an exit in sight plays on");
    for _ in 0..240 { rt.frame(1.0 / 120.0); }
    assert_eq!(exiting(&rt, "b1"), None, "and ends once it has played");
}

/// `n` rows of a dot each in a 100 pt scroll: all but the first few draw nothing.
fn long_list(n: &usize, _: &mut Cx<usize>) -> El {
    let rows = (0..*n).map(|i| col().key(format!("r{i}")).w_full().h(20.0).child(el().size(8.0, 8.0).fill(Color::WHITE)));
    col().w_full().h_full().child(col().key("scroll").w_full().h(100.0).scroll(ScrollSpec { stick_bottom: false }).child(col().children(rows)))
}

/// A window over `rows` rows of [`long_list`], settled, with nothing exiting.
fn settled(rows: usize) -> Runtime<usize> {
    let mut rt = Runtime::new(rows, long_list, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    for _ in 0..600 { if !rt.frame(1.0 / 120.0) { break; } }
    rt
}

/// Microseconds a pass ending the exits nobody sees takes, timed over a hundred passes.
fn pass_us(rt: &mut Runtime<usize>) -> f64 {
    let start = Instant::now();
    for _ in 0..100 { rt.end_unseen_exits(); }
    start.elapsed().as_secs_f64() * 1e4
}

fn median(mut times: Vec<f64>) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

/// Every frame steps motion, and with nothing exiting there is no exit to end: a long list
/// pays for that no more than a short one, where asking each node whether it exits cost each
/// frame in proportion to the nodes that drew nothing.
#[test]
fn with_nothing_exiting_a_long_list_ends_unseen_exits_as_fast_as_a_short_one() {
    let (mut short, mut long) = (settled(40), settled(4_000));
    let (mut a, mut b) = (Vec::new(), Vec::new());
    // Alternated, so load on the machine weighs on both alike.
    for _ in 0..7 {
        a.push(pass_us(&mut short));
        b.push(pass_us(&mut long));
    }
    let (a, b) = (median(a), median(b));
    assert!(b < 10.0 * a, "100× the rows cost {:.0}× as much ({b:.1} µs against {a:.1} µs a pass)", b / a);
}

/// Cost fixture: `cargo test -p rune --profile perf --lib unseen_exits_cost -- --ignored --nocapture`
/// prints the microseconds one frame's pass takes over 100, 400 and 1,600 rows, median of seven.
#[test]
#[ignore = "manual cost fixture: run with --profile perf --ignored --nocapture"]
fn unseen_exits_cost() {
    for rows in [100, 400, 1_600] {
        let mut rt = settled(rows);
        let us = median((0..7).map(|_| pass_us(&mut rt)).collect());
        println!("{rows} rows ({} nodes): {us:.2} µs a pass", rt.nodes.len());
    }
}
