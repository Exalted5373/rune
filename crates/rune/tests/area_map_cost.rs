//! Manual cost fixture for `Area` at map scale: tessellating a world's polygons, picking the one
//! under a point, and panning and zooming them in a window at 2x.
//!
//! `cargo test -p rune --profile perf --test area_map_cost -- --ignored --nocapture` measures
//! 1,400 generated polygons of about 100k points; `RUNE_AREA_GEOJSON=<file>` measures a GeoJSON
//! file instead (Natural Earth's `ne_50m_land.geojson`, say), one area per feature.
use std::time::Instant;

use rune::render::{Gpu, KIND_TRIANGLE, OFFSCREEN_FORMAT, Offscreen, Renderer};
use rune::{Area, Color, Cx, El, Env, FillRule, Live, Path, Runtime, Shape, canvas, el};

const MAP: [f32; 2] = [1440.0, 720.0];

struct Map {
    areas: Vec<Area>,
    offset: Live<[f32; 2]>,
    scale: Live<f32>,
}

fn view(s: &Map, _: &mut Cx<Map>) -> El {
    let shape = s.areas.iter().fold(Shape::new(), |shape, area| shape.fill_area(area, Color::WHITE));
    el().size(1600.0, 800.0).fill(Color::BLACK)
        .child(canvas(shape).size(MAP[0], MAP[1]).absolute(80.0, 40.0).live_offset(&s.offset).live_scale(&s.scale))
}

/// Each feature's rings: from `RUNE_AREA_GEOJSON`, longitude and latitude stretched over `MAP`,
/// or generated: wavy, star-shaped polygons from 16k points down to 8, every fifth with a hole.
fn features() -> (String, Vec<Vec<Vec<[f32; 2]>>>) {
    if let Some(file) = std::env::var_os("RUNE_AREA_GEOJSON") {
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).expect("read the GeoJSON")).expect("GeoJSON");
        let ring = |r: &serde_json::Value| -> Vec<[f32; 2]> {
            r.as_array().expect("a ring").iter().map(|p| {
                let (lon, lat) = (p[0].as_f64().expect("lon") as f32, p[1].as_f64().expect("lat") as f32);
                [(lon + 180.0) / 360.0 * MAP[0], (90.0 - lat) / 180.0 * MAP[1]]
            }).collect()
        };
        let features = json["features"].as_array().expect("features").iter().map(|f| {
            let g = &f["geometry"];
            let polygons = match g["type"].as_str() {
                Some("Polygon") => vec![&g["coordinates"]],
                Some("MultiPolygon") => g["coordinates"].as_array().expect("polygons").iter().collect(),
                other => panic!("geometry {other:?}"),
            };
            polygons.into_iter().flat_map(|p| p.as_array().expect("rings").iter().map(ring)).collect()
        });
        let name = std::path::Path::new(&file).file_name().unwrap_or(&file).to_string_lossy().into_owned();
        return (name, features.collect());
    }
    let wavy = |c: [f32; 2], r: f32, n: usize, phase: f32| -> Vec<[f32; 2]> {
        (0..n).map(|k| {
            let t = std::f32::consts::TAU * k as f32 / n as f32;
            let rr = r * (1.0 + 0.25 * (5.0 * t + phase).sin() + 0.08 * (37.0 * t).sin());
            [c[0] + rr * t.cos(), c[1] + rr * t.sin()]
        }).collect()
    };
    let polygons = (0..1400).map(|i| {
        let n = 8 + (16_000.0 / ((i + 1) as f32).powf(1.1)) as usize;
        let c = [18.0 + (i % 40) as f32 * 36.0, 10.0 + (i / 40) as f32 * 20.0];
        let mut rings = vec![wavy(c, 7.0, n, i as f32)];
        if i % 5 == 0 {
            let mut hole = wavy(c, 3.0, (n / 4).max(8), 0.0);
            hole.reverse();
            rings.push(hole);
        }
        rings
    });
    ("generated".into(), polygons.collect())
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1e3
}

/// GPU milliseconds per frame: `K` draws in one submission, timed from submit to completion.
fn gpu_time(gpu: &Gpu, mut draw: impl FnMut(&mut wgpu::CommandEncoder)) -> f64 {
    const K: u32 = 8;
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    for _ in 0..K {
        draw(&mut enc);
    }
    let commands = enc.finish();
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let t0 = Instant::now();
    gpu.queue.submit([commands]);
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    ms(t0) / f64::from(K)
}

#[test]
#[ignore = "manual cost of filled areas at map scale"]
fn map_scale_areas() {
    let (name, features) = features();
    let paths: Vec<Path> = features.iter().map(|rings| rings.iter().fold(Path::new(), |p, r| p.polygon(r.iter().copied()))).collect();
    let rings: usize = features.iter().map(Vec::len).sum();
    let points: usize = features.iter().flatten().map(Vec::len).sum();
    let (largest, largest_points) = features.iter().map(|f| f.iter().map(Vec::len).sum::<usize>()).enumerate().max_by_key(|e| e.1).unwrap();
    eprintln!("{name}: {} areas, {rings} rings, {points} points; the largest area {largest_points} points", paths.len());

    let mut areas = Vec::new();
    let tessellate = median((0..5).map(|_| {
        let t = Instant::now();
        areas = paths.iter().map(|p| Area::new(p, FillRule::EvenOdd)).collect();
        ms(t)
    }).collect());
    let one = median((0..5).map(|_| {
        let t = Instant::now();
        let _ = Area::new(&paths[largest], FillRule::EvenOdd);
        ms(t)
    }).collect());
    eprintln!("tessellate every area: {tessellate:.1} ms; the largest alone: {one:.2} ms (medians of 5)");

    let mut seed = 0x2545_f491_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32
    };
    let probes: Vec<[f32; 2]> = (0..100_000).map(|_| [next() * MAP[0], next() * MAP[1]]).collect();
    let t = Instant::now();
    let hits = probes.iter().filter(|&&p| areas.iter().any(|a| a.contains(p))).count();
    eprintln!("pick: {:.2} µs a point, testing every area ({hits} of 100000 points inside one)", ms(t) * 1e3 / 1e5);

    let (w, h) = (1600.0f32, 800.0f32);
    let size = ((w * 2.0) as u32, (h * 2.0) as u32);
    let (offset, scale) = (Live::new([0.0, 0.0]), Live::new(1.0));
    let t = Instant::now();
    let mut rt = Runtime::new(Map { areas, offset: offset.clone(), scale: scale.clone() }, view, (w, h), Env::new(false, 2.0));
    rt.frame(0.0);
    let first = ms(t);
    let tris: Vec<_> = rt.instances.iter().filter(|q| q.params[1] == KIND_TRIANGLE).collect();
    // Antialiased edges should be the outlines and nothing else: no seams inside an area.
    let soft: f64 = tris.iter().map(|q| {
        let v = [[q.radii[0], q.radii[1]], [q.radii[2], q.radii[3]], [q.uv[0], q.uv[1]]];
        let mask = q.params[0] as u32;
        (0..3).filter(|e| mask & (1 << e) != 0).map(|e| {
            let (a, b) = (v[e], v[(e + 1) % 3]);
            f64::from(((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt())
        }).sum::<f64>()
    }).sum();
    let perimeter: f64 = features.iter().flatten().map(|r| {
        (0..r.len()).map(|i| {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            f64::from(((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt())
        }).sum::<f64>()
    }).sum();
    let seams = soft / (2.0 * perimeter);
    eprintln!("first frame (build, layout, paint): {first:.1} ms; {} triangle instances; antialiased length / outline length: {seams:.4}", tris.len());

    let gpu = Gpu::headless();
    eprintln!("adapter: {:?}", gpu.adapter.get_info().name);
    let target = Offscreen::new(&gpu.device, size.0, size.1);
    let mut renderer = Renderer::new(&gpu.device, OFFSCREEN_FORMAT);
    let (mut frame_ms, mut encode_ms, mut gpu_ms, mut rebuild_ms) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut step = 0;
    for _ in 0..6 {
        // Panning and zooming through live values: no rebuild, no tessellation.
        let mut frames = Vec::new();
        let g = gpu_time(&gpu, |enc| {
            step += 1;
            let k = step as f32;
            offset.set([(k * 0.37).sin() * 200.0, (k * 0.23).cos() * 100.0]);
            scale.set(1.0 + 0.5 * (k * 0.11).sin().abs());
            let t0 = Instant::now();
            rt.frame(1.0 / 120.0);
            let t1 = Instant::now();
            renderer.prepare(&gpu.device, &gpu.queue, &mut rt.text.atlas, &rt.instances, size);
            renderer.draw(enc, &target.view, [1.0; 4]);
            frames.push(((t1 - t0).as_secs_f64() * 1e3, ms(t1)));
        });
        gpu_ms.push(g);
        frame_ms.extend(frames.iter().map(|f| f.0));
        encode_ms.extend(frames.iter().map(|f| f.1));
        // A view rebuild with the same areas (a hover elsewhere, say): it clones them, shared.
        let t0 = Instant::now();
        rt.update(|_, _| {});
        rt.frame(1.0 / 120.0);
        rebuild_ms.push(ms(t0));
    }
    eprintln!(
        "live pan and zoom at 2x: cpu frame {:.2} ms, prepare+encode {:.2} ms, gpu {:.2} ms; a rebuild with the same areas {:.2} ms (medians)",
        median(frame_ms), median(encode_ms), median(gpu_ms), median(rebuild_ms)
    );
    assert!((seams - 1.0).abs() < 0.01, "interior edges antialiased: {seams}");
}
