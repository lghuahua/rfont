use rfont::Font;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

fn main() {
    println!("🔬 并行 vs 串行性能对比测试\n");

    // 准备测试文件列表
    let test_files: Vec<PathBuf> = (1..=10)
        .map(|i| PathBuf::from(format!("test_bench_{}.ttf", i)))
        .collect();

    let test_text = "阿里巴巴妈妈";

    println!("📊 测试配置:");
    println!("   - 文件数量: {}", test_files.len());
    println!("   - 测试文本: {}", test_text);
    println!("   - 输出格式: WOFF2");
    println!();

    // 串行处理
    println!("⏱️  开始串行处理...");
    let start = Instant::now();
    let mut serial_count = 0;

    for file_path in &test_files {
        if let Ok(font) = Font::load(file_path.to_str().unwrap()) {
            let glyph_ids = font.text_to_glyph_ids(test_text);
            let result = font
                .subset_builder()
                .glyph_ids(glyph_ids)
                .output_format("woff2")
                .compression_level(6)
                .build();

            if result.is_ok() {
                serial_count += 1;
            }
        }
    }

    let serial_duration = start.elapsed();
    println!(
        "✅ 串行处理完成: {} 个文件, 耗时: {:?}",
        serial_count, serial_duration
    );
    println!();

    #[cfg(feature = "parallel")]
    {
        // 并行处理（默认线程数）
        println!("⏱️  开始并行处理（默认线程数）...");
        let start = Instant::now();

        let parallel_default_count: usize = test_files
            .par_iter()
            .filter_map(|file_path| {
                Font::load(file_path.to_str().unwrap()).ok().map(|font| {
                    let glyph_ids = font.text_to_glyph_ids(test_text);
                    font.subset_builder()
                        .glyph_ids(glyph_ids)
                        .output_format("woff2")
                        .compression_level(6)
                        .build()
                        .ok()
                })
            })
            .count();

        let parallel_default_duration = start.elapsed();
        println!(
            "✅ 并行处理完成（默认）: {} 个文件, 耗时: {:?}",
            parallel_default_count, parallel_default_duration
        );
        println!();

        // 并行处理（2 线程）
        println!("⏱️  开始并行处理（2 线程）...");
        let start = Instant::now();

        let parallel_2_count = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build_scoped(
                |thread| thread.run(),
                |pool| {
                    pool.install(|| {
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
                                        .ok()
                                })
                            })
                            .count()
                    })
                },
            )
            .unwrap();

        let parallel_2_duration = start.elapsed();
        println!(
            "✅ 并行处理完成（2 线程）: {} 个文件, 耗时: {:?}",
            parallel_2_count, parallel_2_duration
        );
        println!();

        // 并行处理（4 线程）
        println!("⏱️  开始并行处理（4 线程）...");
        let start = Instant::now();

        let parallel_4_count = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build_scoped(
                |thread| thread.run(),
                |pool| {
                    pool.install(|| {
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
                                        .ok()
                                })
                            })
                            .count()
                    })
                },
            )
            .unwrap();

        let parallel_4_duration = start.elapsed();
        println!(
            "✅ 并行处理完成（4 线程）: {} 个文件, 耗时: {:?}",
            parallel_4_count, parallel_4_duration
        );
        println!();

        // 性能对比总结
        println!("📈 性能对比总结:");
        println!("   串行:     {:?} (基准)", serial_duration);
        println!(
            "   并行(默认): {:?} ({:.1}x)",
            parallel_default_duration,
            serial_duration.as_secs_f64() / parallel_default_duration.as_secs_f64()
        );
        println!(
            "   并行(2线程): {:?} ({:.1}x)",
            parallel_2_duration,
            serial_duration.as_secs_f64() / parallel_2_duration.as_secs_f64()
        );
        println!(
            "   并行(4线程): {:?} ({:.1}x)",
            parallel_4_duration,
            serial_duration.as_secs_f64() / parallel_4_duration.as_secs_f64()
        );
    }

    #[cfg(not(feature = "parallel"))]
    {
        println!("⚠️  Parallel feature 未启用，跳过并行测试");
        println!("   启用方法: cargo run --example perf_comparison --features parallel");
    }
}
