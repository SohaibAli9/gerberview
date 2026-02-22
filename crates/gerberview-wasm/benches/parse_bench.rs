//! Criterion benchmarks for Gerber parse and geometry conversion (PERF-001, PERF-002).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gerberview_wasm::geometry;
use std::io::{BufReader, Cursor};

fn parse_bench(c: &mut Criterion) {
    let data = include_bytes!("../tests/fixtures/kicad-sample/board-F_Cu.gbr");
    let mut group = c.benchmark_group("parse");
    group.sample_size(10);

    // PERF-001: Gerber parse time < 500 ms
    group.bench_function("gerber_parse", |b| {
        b.iter(|| {
            let reader = BufReader::new(Cursor::new(black_box(data.as_slice())));
            black_box(gerber_parser::parse(reader))
        })
    });

    // PERF-002: Geometry conversion time < 1000 ms
    let reader = BufReader::new(Cursor::new(data.as_slice()));
    let doc = match gerber_parser::parse(reader) {
        Ok(d) => d,
        Err((d, _)) => d,
    };
    group.bench_function("geometry_convert", |b| {
        b.iter(|| black_box(geometry::convert(black_box(&doc))))
    });

    group.finish();
}

criterion_group!(benches, parse_bench);
criterion_main!(benches);
