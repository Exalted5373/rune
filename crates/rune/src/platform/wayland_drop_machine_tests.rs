use std::path::PathBuf;

use super::{Drags, Effect, Entering, Event, Landing, Over};
use crate::platform::wayland_drop::Step;

/// Offers are numbers, lists and pipes nothing.
type D = Drags<u32, ()>;
type E = Effect<u32, ()>;

fn step(d: &mut D, event: Event<u32, (), ()>) -> Vec<E> {
    d.step(event).into_iter().collect()
}

fn files(offer: u32, at: (f32, f32)) -> Event<u32, (), ()> {
    Event::Enter { offer: Some(offer), serial: offer * 10, at, entering: Entering::Files(Some(((), ()))) }
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn a_file_drag_enters_once_its_list_is_read_then_moves_and_drops() {
    let mut d = D::default();
    assert_eq!(step(&mut d, files(1, (5.0, 5.0))), [Effect::Take { offer: 1, serial: 10, write: () }]);
    assert_eq!(step(&mut d, Event::Motion((6.0, 7.0))), [], "the host hears nothing before the list");
    let listed = step(&mut d, Event::Listed { landing: false, paths: paths(&["/a.png"]) });
    assert_eq!(listed, [Effect::Host(Step::Enter(paths(&["/a.png"]), (6.0, 7.0)))], "entered where it is now");
    assert_eq!(step(&mut d, Event::Motion((8.0, 9.0))), [Effect::Host(Step::Move((8.0, 9.0)))]);
    let dropped = step(&mut d, Event::Drop);
    assert_eq!(dropped, [Effect::Host(Step::Drop(paths(&["/a.png"]), (8.0, 9.0))), Effect::Finish(1)]);
    assert_eq!(d, D::default());
}

#[test]
fn a_drag_with_no_local_files_is_refused_and_its_drop_only_destroys_the_offer() {
    let mut d = D::default();
    step(&mut d, files(1, (0.0, 0.0)));
    assert_eq!(step(&mut d, Event::Listed { landing: false, paths: Vec::new() }), [Effect::Refuse { offer: 1, serial: 10 }]);
    assert_eq!(step(&mut d, Event::Drop), [Effect::Destroy(1)], "no finish after accepting nothing");
    let no_pipe = Event::Enter { offer: Some(2), serial: 20, at: (0.0, 0.0), entering: Entering::Files(None) };
    assert_eq!(step(&mut d, no_pipe), [Effect::Refuse { offer: 2, serial: 20 }]);
    assert_eq!(d.over, Over::Refused(2));
}

#[test]
fn other_drags_are_ignored_and_a_new_enter_ends_the_last_drag() {
    let mut d = D::default();
    let text = Event::Enter { offer: Some(1), serial: 10, at: (0.0, 0.0), entering: Entering::Elsewhere };
    assert_eq!(step(&mut d, text), []);
    assert_eq!(step(&mut d, files(2, (1.0, 1.0))), [Effect::Destroy(1), Effect::Take { offer: 2, serial: 20, write: () }]);
    step(&mut d, Event::Listed { landing: false, paths: paths(&["/b"]) });
    let again = step(&mut d, files(3, (2.0, 2.0)));
    assert_eq!(again, [Effect::Host(Step::Leave), Effect::Destroy(2), Effect::Take { offer: 3, serial: 30, write: () }]);
    assert_eq!(step(&mut d, Event::Leave), [Effect::Destroy(3)], "the host never heard 3 enter");
}

#[test]
fn a_drop_before_the_list_is_read_lands_when_it_is() {
    let mut d = D::default();
    step(&mut d, files(1, (3.0, 4.0)));
    assert_eq!(step(&mut d, Event::Drop), []);
    assert_eq!(d.landing, Landing::Reading { offer: 1, at: (3.0, 4.0), list: () });
    assert!(d.list_mut(true).is_some() && d.list_mut(false).is_none());
    step(&mut d, files(2, (0.0, 0.0)));
    let landed = step(&mut d, Event::Listed { landing: true, paths: paths(&["/c"]) });
    assert_eq!(landed, [Effect::Host(Step::Drop(paths(&["/c"]), (3.0, 4.0))), Effect::Finish(1)]);
    assert!(matches!(d.over, Over::Reading { offer: 2, .. }), "the next drag goes on");
    assert_eq!(step(&mut d, Event::Stop), [Effect::Destroy(2)]);
    assert_eq!(d, D::default());
}
