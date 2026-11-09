//! Where a Wayland file drag over a window stands, kept apart from the compositor calls in
//! `platform/wayland_drop.rs` so its steps are plain data: which drag is over the window and
//! whether the host heard it enter, and a dropped drag whose file list is still being read.
//! Generic over the offer (`O`), the list being read (`L`) and the pipe's write end the source
//! writes it to (`W`), so its tests need no compositor.

use std::path::PathBuf;

use super::Step;
use crate::machine::Effects;

type Point = (f32, f32);

/// The drag over the window's surface.
#[derive(Debug, PartialEq)]
pub(crate) enum Over<O, L> {
    Idle,
    /// A drag this doesn't take: it offers no files, or it's over another surface.
    Ignored(O),
    /// A file drag that entered; its list is being read, so the host hasn't heard of it.
    Reading { offer: O, serial: u32, at: Point, list: L },
    /// One whose list named no local files (or couldn't be read): the source shows it can't
    /// drop here.
    Refused(O),
    /// One the host heard enter, with its files.
    Entered { offer: O, at: Point, paths: Vec<PathBuf> },
}

/// A drag dropped before its file list was read, until it is.
#[derive(Debug, PartialEq)]
pub(crate) enum Landing<O, L> {
    Idle,
    Reading { offer: O, at: Point, list: L },
}

/// Both: a new drag can come over the window while the last one's list is still arriving.
#[derive(Debug, PartialEq)]
pub(crate) struct Drags<O, L> {
    pub(crate) over: Over<O, L>,
    pub(crate) landing: Landing<O, L>,
}

// By hand: a derive would ask the offer and the list to have defaults too.
impl<O, L> Default for Over<O, L> {
    fn default() -> Self { Over::Idle }
}

impl<O, L> Default for Landing<O, L> {
    fn default() -> Self { Landing::Idle }
}

impl<O, L> Default for Drags<O, L> {
    fn default() -> Self { Drags { over: Over::Idle, landing: Landing::Idle } }
}

/// How a drag that entered can be read.
#[derive(Debug, PartialEq)]
pub(crate) enum Entering<L, W> {
    /// Not a drag this takes: no `text/uri-list`, or over another surface.
    Elsewhere,
    /// A file drag over the window, with the pipe its list comes through: the read end and the
    /// write end the source gets. None when no pipe could be made.
    Files(Option<(L, W)>),
}

/// What the compositor and the list reader said.
#[derive(Debug, PartialEq)]
pub(crate) enum Event<O, L, W> {
    Enter { offer: Option<O>, serial: u32, at: Point, entering: Entering<L, W> },
    Motion(Point),
    /// The drag left the surface or was cancelled.
    Leave,
    Drop,
    /// The file list of the drag over the window (`landing: false`) or of the dropped one was
    /// read to its end.
    Listed { landing: bool, paths: Vec<PathBuf> },
    /// The thread stops: the window is going.
    Stop,
}

/// What the caller does, in order.
#[derive(Debug, PartialEq)]
pub(crate) enum Effect<O, W> {
    /// Accept `text/uri-list`, ask for a copy, and have the source write the list to `write`.
    Take { offer: O, serial: u32, write: W },
    /// Accept nothing, so the source shows it can't drop.
    Refuse { offer: O, serial: u32 },
    Destroy(O),
    /// Tell the source the drop is done, then destroy the offer.
    Finish(O),
    /// A step for the host.
    Host(Step),
}

impl<O: Clone, L> Drags<O, L> {
    /// The list being read: the drag over the window's, or the dropped one's.
    pub(crate) fn list_mut(&mut self, landing: bool) -> Option<&mut L> {
        match (landing, &mut self.over, &mut self.landing) {
            (false, Over::Reading { list, .. }, _) | (true, _, Landing::Reading { list, .. }) => Some(list),
            _ => None,
        }
    }

    /// The drags after `event`, and what the caller does.
    pub(crate) fn step<W>(&mut self, event: Event<O, L, W>) -> Effects<Effect<O, W>> {
        match event {
            Event::Enter { offer, serial, at, entering } => {
                let mut effects = self.leave();
                let Some(offer) = offer else { return effects };
                self.over = match entering {
                    Entering::Elsewhere => Over::Ignored(offer),
                    Entering::Files(Some((list, write))) => {
                        effects.push(Effect::Take { offer: offer.clone(), serial, write });
                        Over::Reading { offer, serial, at, list }
                    }
                    Entering::Files(None) => {
                        effects.push(Effect::Refuse { offer: offer.clone(), serial });
                        Over::Refused(offer)
                    }
                };
                effects
            }
            Event::Motion(to) => match &mut self.over {
                Over::Reading { at, .. } => {
                    *at = to;
                    Effects::none()
                }
                Over::Entered { at, .. } => {
                    *at = to;
                    Effects::one(Effect::Host(Step::Move(to)))
                }
                _ => Effects::none(),
            },
            Event::Leave => self.leave(),
            Event::Drop => match std::mem::take(&mut self.over) {
                Over::Idle => Effects::none(),
                // Accepting nothing makes a drop a cancel; a compositor that sends it anyway
                // gets no `finish`, which would be an error after a null accept.
                Over::Ignored(offer) | Over::Refused(offer) => Effects::one(Effect::Destroy(offer)),
                Over::Reading { offer, at, list, .. } => {
                    match std::mem::replace(&mut self.landing, Landing::Reading { offer, at, list }) {
                        Landing::Reading { offer, .. } => Effects::one(Effect::Destroy(offer)),
                        Landing::Idle => Effects::none(),
                    }
                }
                Over::Entered { offer, at, paths } => Effects::two(Effect::Host(Step::Drop(paths, at)), Effect::Finish(offer)),
            },
            Event::Listed { landing: false, paths } => match std::mem::take(&mut self.over) {
                Over::Reading { offer, serial, .. } if paths.is_empty() => {
                    self.over = Over::Refused(offer.clone());
                    Effects::one(Effect::Refuse { offer, serial })
                }
                Over::Reading { offer, at, .. } => {
                    let step = Step::Enter(paths.clone(), at);
                    self.over = Over::Entered { offer, at, paths };
                    Effects::one(Effect::Host(step))
                }
                other => {
                    self.over = other;
                    Effects::none()
                }
            },
            Event::Listed { landing: true, paths } => match std::mem::take(&mut self.landing) {
                Landing::Reading { offer, .. } if paths.is_empty() => Effects::one(Effect::Finish(offer)),
                Landing::Reading { offer, at, .. } => Effects::two(Effect::Host(Step::Drop(paths, at)), Effect::Finish(offer)),
                Landing::Idle => Effects::none(),
            },
            Event::Stop => {
                let mut effects = self.leave();
                if let Landing::Reading { offer, .. } = std::mem::take(&mut self.landing) {
                    effects.push(Effect::Destroy(offer));
                }
                effects
            }
        }
    }

    /// The drag over the window goes: the host hears it leave if it heard it enter.
    fn leave<W>(&mut self) -> Effects<Effect<O, W>> {
        match std::mem::take(&mut self.over) {
            Over::Idle => Effects::none(),
            Over::Entered { offer, .. } => Effects::two(Effect::Host(Step::Leave), Effect::Destroy(offer)),
            Over::Ignored(offer) | Over::Refused(offer) | Over::Reading { offer, .. } => Effects::one(Effect::Destroy(offer)),
        }
    }
}

#[cfg(test)]
#[path = "wayland_drop_machine_tests.rs"]
mod tests;
