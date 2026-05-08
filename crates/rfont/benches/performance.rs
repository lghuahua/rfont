use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rfont::Font;
use std::time::Duration;

fn benchmark_font_loading(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";

    let mut group = c.benchmark_group("font_loading");
    group.sample_size(150); // 增加采样数提高准确性
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("load_ttf_font", |b| {
        b.iter(|| Font::load(black_box(font_path)).unwrap())
    });

    group.finish();
}

fn benchmark_cmap_lookup(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_chars = vec!['阿', '里', '妈', '中', '文', '测', '试'];

    let mut group = c.benchmark_group("cmap_lookup");
    group.sample_size(200); // cmap 查找很快，需要更多采样
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("without_cache", |b| {
        b.iter(|| {
            for &ch in &test_chars {
                black_box(font.cmap.get_glyph_id(ch));
            }
        })
    });

    group.finish();
}

fn benchmark_text_to_glyphs(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_text = "阿里巴巴妈妈中文测试";

    let mut group = c.benchmark_group("text_conversion");
    group.sample_size(150);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("text_to_glyph_ids", |b| {
        b.iter(|| black_box(font.text_to_glyph_ids(test_text)))
    });

    group.finish();
}

fn benchmark_glyph_iteration(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();

    let mut group = c.benchmark_group("glyph_iteration");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("glyph_iterator", |b| {
        b.iter(|| {
            let mut count = 0;
            for (_id, _data) in font.glyph_iter() {
                count += 1;
            }
            black_box(count)
        })
    });

    group.finish();
}

fn benchmark_subset_creation(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_text = "阿里巴巴";

    let mut group = c.benchmark_group("subset_creation");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(5)); // 子集化较慢，增加预热
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("with_text", |b| {
        b.iter(|| {
            font.subset_builder()
                .text(black_box(test_text))
                .build()
                .unwrap()
        })
    });

    group.finish();
}

fn benchmark_chunked_processing(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();

    let mut group = c.benchmark_group("chunked_processing");
    group.sample_size(150);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("get_glyphs_chunked_100", |b| {
        b.iter(|| {
            let mut total = 0;
            font.get_glyphs_chunked(100, |chunk| {
                total += chunk.len();
                Ok::<(), rfont_types::FontError>(())
            })
            .unwrap();
            black_box(total)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_font_loading,
    benchmark_cmap_lookup,
    benchmark_text_to_glyphs,
    benchmark_glyph_iteration,
    benchmark_subset_creation,
    benchmark_chunked_processing,
    benchmark_parallel_vs_serial
);
criterion_main!(benches);

/// 并行 vs 串行性能对比测试
#[cfg(feature = "parallel")]
fn benchmark_parallel_vs_serial(c: &mut Criterion) {
    use rayon::prelude::*;

    // 准备测试文件列表
    let test_files: Vec<PathBuf> = (1..=10)
        .map(|i| PathBuf::from(format!("test_bench_{}.ttf", i)))
        .collect();

    let test_text = "阿里巴巴妈妈";

    // 串行处理基准测试
    c.bench_function("serial_batch_subset_10_files", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for file_path in &test_files {
                if let Ok(font) = Font::load(file_path.to_str().unwrap()) {
                    let glyph_ids = font.text_to_glyph_ids(test_text);
                    let result = font
                        .subset_builder()
                        .glyph_ids(glyph_ids)
                        .output_format("woff2")
                        .compression_level(6)
                        .build();
                    results.push(result);
                }
            }
            black_box(results.len())
        })
    });

    // 并行处理基准测试（默认线程数）
    c.bench_function("parallel_batch_subset_10_files_default", |b| {
        b.iter(|| {
            let results: Vec<_> = test_files
                .par_iter()
                .filter_map(|file_path| {
                    Font::load(file_path.to_str().unwrap()).ok().map(|font| {
                        let glyph_ids = font.text_to_glyph_ids(test_text);
                        font.subset_builder()
                            .glyph_ids(glyph_ids)
                            .output_format("woff2")
                            .compression_level(6)
                            .build()
                    })
                })
                .collect();
            black_box(results.len())
        })
    });

    // 并行处理基准测试（2 线程）
    c.bench_function("parallel_batch_subset_10_files_2_threads", |b| {
        b.iter(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build_scoped(
                    |thread| thread.run(),
                    |pool| {
                        let results: Vec<_> = pool.install(|| {
                            test_files
                                .par_iter()
                                .filter_map(|file_path| {
                                    Font::load(file_path.to_str().unwrap()).ok().map(|font| {
                                        let glyph_ids = font.text_to_glyph_ids(test_text);
                                        font.subset_builder()
                                            .glyph_ids(glyph_ids)
                                            .output_format("woff2")
                                            .compression_level(6)
                                            .build()
                                    })
                                })
                                .collect()
                        });
                        black_box(results.len())
                    },
                )
                .unwrap()
        })
    });

    // 并行处理基准测试（4 线程）
    c.bench_function("parallel_batch_subset_10_files_4_threads", |b| {
        b.iter(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build_scoped(
                    |thread| thread.run(),
                    |pool| {
                        let results: Vec<_> = pool.install(|| {
                            test_files
                                .par_iter()
                                .filter_map(|file_path| {
                                    Font::load(file_path.to_str().unwrap()).ok().map(|font| {
                                        let glyph_ids = font.text_to_glyph_ids(test_text);
                                        font.subset_builder()
                                            .glyph_ids(glyph_ids)
                                            .output_format("woff2")
                                            .compression_level(6)
                                            .build()
                                    })
                                })
                                .collect()
                        });
                        black_box(results.len())
                    },
                )
                .unwrap()
        })
    });
}

#[cfg(not(feature = "parallel"))]
fn benchmark_parallel_vs_serial(_c: &mut Criterion) {
    // 未启用 parallel feature 时，此测试不执行
    println!("⚠️  Parallel benchmarks skipped (enable with --features parallel)");
}
