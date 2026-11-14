use std::sync::Arc;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};

use super::ImageCache;
use crate::{Cx, El, Env, Pixels, Runtime, el};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A looping 2×2 GIF: red for 100 ms, then blue for 100 ms.
fn red_then_blue() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder.set_repeat(Repeat::Infinite).unwrap();
        for color in [RED, BLUE] {
            let frame = RgbaImage::from_fn(2, 2, |_, _| image::Rgba(color));
            encoder.encode_frame(Frame::from_parts(frame, 0, 0, Delay::from_numer_denom_ms(100, 1))).unwrap();
        }
    }
    bytes
}

fn view(s: &Pixels, _: &mut Cx<Pixels>) -> El {
    el().size(2.0, 2.0).pixels(s)
}

/// Whether the image atlas holds a pixel of `color`.
fn shows(rt: &Runtime<Pixels>, color: [u8; 4]) -> bool {
    rt.images.pages.iter().any(|p| p.data.chunks(4).any(|px| px == color))
}

/// Plays a frame 150 ms in, past the GIF's first frame.
fn played(reduced: bool) -> Runtime<Pixels> {
    let cache = ImageCache::new(Arc::new(|| {}));
    let image = cache.load(red_then_blue());
    image.wait().unwrap();
    let mut rt = Runtime::new(image.pixels().clone(), view, (2.0, 2.0), Env::new(true, 1.0));
    rt.set_system_reduced_motion(reduced);
    rt.frame(0.0);
    rt.frame(0.15);
    rt
}

#[test]
fn an_animated_image_advances_and_wakes_for_its_next_frame() {
    let rt = played(false);
    assert!(shows(&rt, BLUE), "150 ms in, the second frame shows");
    assert!(rt.motion_wake_deadline().is_some(), "it wakes the window for the next frame");
}

/// As a spinner holds a pose, an animation under reduced motion shows its first frame and asks
/// for no frames.
#[test]
fn under_reduced_motion_an_animated_image_holds_its_first_frame() {
    let rt = played(true);
    assert!(shows(&rt, RED) && !shows(&rt, BLUE), "the first frame stays");
    assert_eq!(rt.motion_wake_deadline(), None, "nothing wakes the window");
}
