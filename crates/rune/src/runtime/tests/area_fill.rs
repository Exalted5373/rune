use super::*;
use crate::{Area, FillRule, Path, Shape, canvas};

fn square(x: f32, y: f32, side: f32) -> [[f32; 2]; 4] {
    [[x, y], [x + side, y], [x + side, y + side], [x, y + side]]
}

/// The area in a canvas, panned `.0` points to the right.
fn view(s: &(f32, Area), _: &mut Cx<(f32, Area)>) -> El {
    el().size(400.0, 200.0).child(canvas(Shape::new().fill_area(&s.1, crate::Color::WHITE)).size(120.0, 120.0).absolute(s.0, 10.0))
}

fn triangles(rt: &Runtime<(f32, Area)>) -> Vec<[[f32; 2]; 3]> {
    rt.instances
        .iter()
        .filter(|q| q.params[1] == KIND_TRIANGLE)
        .map(|q| [[q.radii[0], q.radii[1]], [q.radii[2], q.radii[3]], [q.uv[0], q.uv[1]]])
        .collect()
}

fn holds(t: &[[f32; 2]; 3], p: [f32; 2]) -> bool {
    let side = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let s = [side(t[0], t[1]), side(t[1], t[2]), side(t[2], t[0])];
    s.iter().all(|&d| d > 0.0) || s.iter().all(|&d| d < 0.0)
}

#[test]
fn an_area_paints_round_its_hole_and_pans_with_the_same_triangles() {
    let frame = Area::new(&Path::new().polygon(square(0.0, 0.0, 120.0)).polygon(square(40.0, 40.0, 40.0)), FillRule::EvenOdd);
    let mut rt = Runtime::new((0.0, frame), view, (400.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let at_rest = triangles(&rt);
    assert_eq!(at_rest.len(), 8);
    assert!(!at_rest.iter().any(|t| holds(t, [61.3, 72.1])), "nothing paints the hole");
    assert_eq!(at_rest.iter().filter(|t| holds(t, [21.3, 17.1])).count(), 1, "one triangle owns a point of the frame");

    rt.update(|s, _| s.0 = 100.0);
    // A box that moves glides there; let it land.
    for _ in 0..240 {
        rt.frame(1.0 / 120.0);
    }
    let panned = triangles(&rt);
    let moved: Vec<[[f32; 2]; 3]> = at_rest.iter().map(|t| t.map(|p| [p[0] + 100.0, p[1]])).collect();
    assert_eq!(panned, moved, "the same triangles, moved");
}
