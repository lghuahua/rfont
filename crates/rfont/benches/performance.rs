use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rfont::Font;

fn benchmark_font_loading(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    
    c.bench_function("load_ttf_font", |b| {
        b.iter(|| {
            Font::load(black_box(font_path)).unwrap()
        })
    });
}

fn benchmark_cmap_lookup(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_chars = vec!['阿', '里', '妈', '中', '文', '测', '试'];
    
    c.bench_function("cmap_lookup_without_cache", |b| {
        b.iter(|| {
            for &ch in &test_chars {
                black_box(font.cmap.get_glyph_id(ch));
            }
        })
    });
}

fn benchmark_text_to_glyphs(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_text = "阿里巴巴妈妈中文测试";
    
    c.bench_function("text_to_glyph_ids", |b| {
        b.iter(|| {
            black_box(font.text_to_glyph_ids(test_text))
        })
    });
}

fn benchmark_glyph_iteration(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    
    c.bench_function("glyph_iterator", |b| {
        b.iter(|| {
            let mut count = 0;
            for (_id, _data) in font.glyph_iter() {
                count += 1;
            }
            black_box(count)
        })
    });
}

fn benchmark_subset_creation(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    let test_text = "阿里巴巴";
    
    c.bench_function("subset_with_text", |b| {
        b.iter(|| {
            font.subset_builder()
                .text(black_box(test_text))
                .build()
                .unwrap()
        })
    });
}

fn benchmark_chunked_processing(c: &mut Criterion) {
    let font_path = "src/AlimamaDaoLiTi.ttf";
    let font = Font::load(font_path).unwrap();
    
    c.bench_function("get_glyphs_chunked_100", |b| {
        b.iter(|| {
            let mut total = 0;
            font.get_glyphs_chunked(100, |chunk| {
                total += chunk.len();
                Ok::<(), rfont_types::FontError>(())
            }).unwrap();
            black_box(total)
        })
    });
}

criterion_group!(
    benches,
    benchmark_font_loading,
    benchmark_cmap_lookup,
    benchmark_text_to_glyphs,
    benchmark_glyph_iteration,
    benchmark_subset_creation,
    benchmark_chunked_processing
);
criterion_main!(benches);
