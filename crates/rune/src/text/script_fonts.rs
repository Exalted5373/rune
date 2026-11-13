//! Scripts an app maps to its own fonts. Text in a mapped script tries them after the element's
//! own families and before the fallbacks Rune adds: the monospace script faces, symbol and math
//! families a stack ends with, and the system's script fonts loaded on first use
//! (`fallback::SCRIPT_FONTS`).

use std::ops::Range;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use parley::FontFamilyName;
use unicode_segmentation::UnicodeSegmentation;

use super::TextSystem;
use super::fallback::{ARABIC, BENGALI, DEVANAGARI, HEBREW, THAI};

static CHOSEN: Mutex<Vec<([u8; 4], String)>> = Mutex::new(Vec::new());
/// Counts changes to `CHOSEN`, so a text system notices one with an atomic load.
static EPOCH: AtomicU64 = AtomicU64::new(0);

/// The scripts an app can map, by ISO 15924 tag, and the blocks of their letters and marks.
const SCRIPTS: &[(&[u8; 4], &[(u32, u32)])] = &[
    (b"Arab", ARABIC),
    (b"Armn", &[(0x0530, 0x058F), (0xFB13, 0xFB17)]),
    (b"Beng", BENGALI),
    (b"Copt", &[(0x2C80, 0x2CFF), (0x102E0, 0x102FF)]),
    (b"Cyrl", &[(0x0400, 0x052F), (0x1C80, 0x1C8F), (0x2DE0, 0x2DFF), (0xA640, 0xA69F)]),
    (b"Deva", DEVANAGARI),
    (b"Ethi", &[(0x1200, 0x139F), (0x2D80, 0x2DDF), (0xAB00, 0xAB2F), (0x1E7E0, 0x1E7FF)]),
    (b"Grek", &[(0x0370, 0x03FF), (0x1F00, 0x1FFF)]),
    (b"Hebr", HEBREW),
    (b"Samr", &[(0x0800, 0x083F)]),
    (b"Syrc", &[(0x0700, 0x074F), (0x0860, 0x086F)]),
    (b"Thai", THAI),
];

/// Maps scripts to the app's own fonts in every text system. Each pair is an ISO 15924 script
/// tag and a CSS family list naming fonts the app registered (`AppSpec::fonts`). Text in that
/// script which the element's own families (its family, then the sans slot behind a named one)
/// don't draw takes the first of those families that does, before any system font. Mappable:
/// `Arab`, `Armn`, `Beng`, `Copt`, `Cyrl`, `Deva`, `Ethi`, `Grek`, `Hebr`, `Samr`, `Syrc` and
/// `Thai` (any case); other tags are skipped. Each call replaces the whole map, and text systems
/// drop what they shaped under the old one before their next layout. An app calls it once,
/// before its first window opens.
///
/// ```no_run
/// rune::text::set_script_fonts(&[("Hebr", "Noto Serif Hebrew"), ("Grek", "Gentium Plus")]);
/// ```
pub fn set_script_fonts(fonts: &[(&str, &str)]) {
    let fonts = parse(fonts);
    let mut chosen = CHOSEN.lock().unwrap_or_else(|e| e.into_inner());
    if *chosen != fonts {
        *chosen = fonts;
        EPOCH.fetch_add(1, Ordering::Relaxed);
    }
}

/// The mappable pairs of `fonts`, tags title-cased.
fn parse(fonts: &[(&str, &str)]) -> Vec<([u8; 4], String)> {
    fonts
        .iter()
        .filter_map(|&(tag, families)| {
            let tag = SCRIPTS.iter().find(|(known, _)| tag.as_bytes().eq_ignore_ascii_case(&known[..]))?.0;
            Some((*tag, families.to_owned()))
        })
        .collect()
}

/// What a text system took up from [`set_script_fonts`] or [`TextSystem::set_script_fonts`].
#[derive(Default)]
pub(super) struct ScriptMap {
    epoch: u64,
    fonts: Vec<([u8; 4], String)>,
}

impl ScriptMap {
    /// The byte ranges of `text` in a mapped script, whole graphemes by their first letter,
    /// with that script's family list; neighbours in one script merge.
    fn routes(&self, text: &str) -> Vec<(Range<usize>, &str)> {
        let mut out: Vec<(Range<usize>, &str)> = Vec::new();
        if self.fonts.is_empty() || text.is_ascii() {
            return out;
        }
        for (i, g) in text.grapheme_indices(true) {
            let Some(ch) = g.chars().next().filter(|c| !c.is_ascii()) else { continue };
            let Some((_, families)) = self.fonts.iter().find(|(tag, _)| covers(tag, ch)) else { continue };
            match out.last_mut() {
                Some((last, f)) if last.end == i && std::ptr::eq(*f, families.as_str()) => last.end = i + g.len(),
                _ => out.push((i..i + g.len(), families.as_str())),
            }
        }
        out
    }

    /// Font stacks for the ranges of `text` in a mapped script: the stack each byte would take
    /// and how many fallbacks end it (`own`), with the script's families before those
    /// fallbacks, cut where `spans` start or end.
    pub(super) fn stacks<'a, 'l>(
        &'a self,
        text: &str,
        spans: &[super::Span],
        own: impl Fn(usize) -> (&'l [FontFamilyName<'a>], usize),
    ) -> Vec<(Range<usize>, Vec<FontFamilyName<'a>>)>
    where
        'a: 'l,
    {
        let mut out = Vec::new();
        for (range, families) in self.routes(text) {
            let mut cuts: Vec<usize> =
                spans.iter().flat_map(|s| [s.start as usize, s.end as usize]).filter(|c| range.contains(c)).chain([range.start, range.end]).collect();
            cuts.sort_unstable();
            cuts.dedup();
            for w in cuts.windows(2) {
                let (base, tail) = own(w[0]);
                let at = base.len().saturating_sub(tail);
                let stack = base[..at].iter().cloned().chain(FontFamilyName::parse_css_list(families).flatten()).chain(base[at..].iter().cloned());
                out.push((w[0]..w[1], stack.collect()));
            }
        }
        out
    }
}

fn covers(tag: &[u8; 4], ch: char) -> bool {
    let blocks = SCRIPTS.iter().find(|(t, _)| *t == tag).map_or(&[][..], |s| s.1);
    blocks.iter().any(|&(lo, hi)| (lo..=hi).contains(&(ch as u32)))
}

impl TextSystem {
    /// Maps scripts to registered families for this text system alone, as [`set_script_fonts`]
    /// does for every one, until a later [`set_script_fonts`] replaces it. False, dropping
    /// nothing, when it already has this map.
    pub fn set_script_fonts(&mut self, fonts: &[(&str, &str)]) -> bool {
        self.apply_script_fonts(parse(fonts))
    }

    fn apply_script_fonts(&mut self, fonts: Vec<([u8; 4], String)>) -> bool {
        if self.script_fonts.fonts == fonts {
            return false;
        }
        self.script_fonts.fonts = fonts;
        self.drop_shaped();
        true
    }

    /// Takes up the map [`set_script_fonts`] chose since this text system last looked.
    pub(super) fn follow_script_fonts(&mut self) {
        let epoch = EPOCH.load(Ordering::Relaxed);
        if epoch == self.script_fonts.epoch {
            return;
        }
        self.script_fonts.epoch = epoch;
        let chosen = CHOSEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
        self.apply_script_fonts(chosen);
    }
}
