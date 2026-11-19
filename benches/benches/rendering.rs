//! Benchmarks for the hot paths of the conversion pipeline.
//!
//! Run with `cargo bench -p benches`. Record results in `docs/PROGRESS.md`
//! together with hardware, Rust version, and grid sizes.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use renderer_core::{
    GridSpec, ProcessingConfig, Rgb, RgbImage, luminance, palette::Palette, render_frame,
};

/// Builds a deterministic gradient image, so runs are comparable.
fn gradient(width: u32, height: u32) -> RgbImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(Rgb::new(
                (x % 256) as u8,
                (y % 256) as u8,
                ((x + y) % 256) as u8,
            ));
        }
    }
    RgbImage::new(width, height, pixels).unwrap()
}

fn bench_luminance(c: &mut Criterion) {
    let samples: Vec<Rgb> = (0..=255u8).map(|v| Rgb::new(v, 255 - v, v / 2)).collect();

    c.bench_function("luminance/256_samples", |b| {
        b.iter(|| {
            let mut acc = 0u32;
            for &px in &samples {
                acc += luminance(black_box(px)) as u32;
            }
            acc
        });
    });
}

fn bench_palette_mapping(c: &mut Criterion) {
    let palette = Palette::default_palette();

    c.bench_function("palette/map_256_luminances", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for lum in 0..=255u8 {
                acc += palette.map(black_box(lum)) as usize;
            }
            acc
        });
    });
}

fn bench_render_frame(c: &mut Criterion) {
    // 1080p source, the largest input the spec asks us to test.
    let image = gradient(1920, 1080);

    let mut group = c.benchmark_group("render_frame/1080p_source");

    for (label, grid) in [
        (
            "60x20",
            GridSpec {
                width: 60,
                height: 20,
            },
        ),
        (
            "100x30",
            GridSpec {
                width: 100,
                height: 30,
            },
        ),
        (
            "160x45",
            GridSpec {
                width: 160,
                height: 45,
            },
        ),
    ] {
        group.bench_with_input(BenchmarkId::new("box", label), &grid, |b, &grid| {
            let config = ProcessingConfig::default();
            b.iter(|| render_frame(black_box(&image), grid, &config, 0));
        });

        group.bench_with_input(BenchmarkId::new("point", label), &grid, |b, &grid| {
            let config = ProcessingConfig {
                sampling: renderer_core::SamplingMode::Point,
                ..Default::default()
            };
            b.iter(|| render_frame(black_box(&image), grid, &config, 0));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_luminance,
    bench_palette_mapping,
    bench_render_frame
);
criterion_main!(benches);
