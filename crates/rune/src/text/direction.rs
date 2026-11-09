//! The base direction of a text's paragraphs.

/// The direction a text's paragraphs run in, which sets how bidirectional text orders its
/// runs and which side `Align::Start` lines hug. `Auto` takes each paragraph's direction from
/// its first strong letter (Hebrew and Arabic run right to left, Latin left to right).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

impl TextDirection {
    pub(crate) fn base(self) -> parley::BaseDirection {
        match self {
            TextDirection::Auto => parley::BaseDirection::Auto,
            TextDirection::Ltr => parley::BaseDirection::Ltr,
            TextDirection::Rtl => parley::BaseDirection::Rtl,
        }
    }
}
