//! 从 `glyf` / `loca` 表中提取字形轮廓

use rfont::Font;
use rfont_core::tables::glyf::{
    ARG_1_AND_2_ARE_WORDS, ARGS_ARE_XY_VALUES, CompositeComponent, FLAG_ON_CURVE, GlyphData,
    SimpleGlyph, WE_HAVE_A_SCALE, WE_HAVE_A_TWO_BY_TWO, WE_HAVE_AN_X_AND_Y_SCALE,
};
use rfont_core::tables::glyf_lazy::GlyfLazyLoader;
use rfont_types::{FontError, Reader, Tag};

const GLYF: Tag = Tag(*b"glyf");
/// 复合字形嵌套上限，超过视为异常数据，避免递归失控
const MAX_COMPONENT_DEPTH: usize = 8;
/// F2Dot14 定点数的小数位数
const F2DOT14_SCALE: f64 = 16384.0;

/// 轮廓上的一个点，坐标已换算到字体单位（Y 轴向上）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    /// 是否落在曲线上；false 表示二次贝塞尔的控制点
    pub on_curve: bool,
}

/// 单个字形的轮廓
#[derive(Debug, Clone, Default)]
pub struct GlyphOutline {
    /// 闭合轮廓，每个轮廓按 TrueType 顺序排列
    pub contours: Vec<Vec<Point>>,
    /// 步进宽度（字体单位）
    pub advance_width: u16,
}

impl GlyphOutline {
    /// 轮廓包围盒 `(min_x, min_y, max_x, max_y)`，空心字形返回 None
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let mut iter = self.contours.iter().flatten();
        let first = iter.next()?;

        let mut bounds = (first.x, first.y, first.x, first.y);
        for point in iter {
            bounds.0 = bounds.0.min(point.x);
            bounds.1 = bounds.1.min(point.y);
            bounds.2 = bounds.2.max(point.x);
            bounds.3 = bounds.3.max(point.y);
        }
        Some(bounds)
    }

    /// 平移全部轮廓（用于文本排版）
    pub fn translate(&mut self, dx: f64, dy: f64) {
        for point in self.contours.iter_mut().flatten() {
            point.x += dx;
            point.y += dy;
        }
    }
}

/// 仿射变换 `[a b c d e f]`，映射为 `x' = a·x + c·y + e`、`y' = b·x + d·y + f`
#[derive(Debug, Clone, Copy)]
struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Transform {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// 以 `self` 为外层、`inner` 为内层做复合
    fn combine(&self, inner: &Transform) -> Transform {
        Transform {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }
}

/// 提取单个字形的轮廓
///
/// 复合字形会递归展开，并应用组件的平移、缩放与 2x2 变换；
/// 组件使用点匹配（`ARGS_ARE_XY_VALUES` 未置位）时按锚点对齐。
///
/// 仅支持 TrueType `glyf` 轮廓，CFF/CFF2 字体返回错误。
pub fn glyph_outline(font: &Font, glyph_id: u16) -> Result<GlyphOutline, FontError> {
    let glyf = font.font_data().get_table_bytes(GLYF).ok_or_else(|| {
        FontError::Generic(
            "当前仅支持 TrueType glyf 轮廓，CFF/CFF2 字体暂不支持导出 SVG".to_string(),
        )
    })?;
    let loader = GlyfLazyLoader::new(glyf, &font.loca().offsets)?;

    let mut contours = Vec::new();
    let mut stack = Vec::new();
    collect(
        &loader,
        glyph_id,
        Transform::IDENTITY,
        &mut contours,
        &mut stack,
    )?;

    let advance_width = font
        .hmtx()
        .get_metric(glyph_id)
        .map(|metric| metric.advance_width)
        .unwrap_or(0);

    Ok(GlyphOutline {
        contours,
        advance_width,
    })
}

fn collect(
    loader: &GlyfLazyLoader<'_>,
    glyph_id: u16,
    transform: Transform,
    out: &mut Vec<Vec<Point>>,
    stack: &mut Vec<u16>,
) -> Result<(), FontError> {
    if stack.len() >= MAX_COMPONENT_DEPTH || stack.contains(&glyph_id) {
        return Ok(());
    }

    let record = loader.load_glyph(glyph_id)?;
    match record.data {
        GlyphData::Empty => Ok(()),
        GlyphData::Simple(simple) => {
            push_simple(&simple, &transform, out);
            Ok(())
        }
        GlyphData::Composite(composite) => {
            stack.push(glyph_id);
            for component in &composite.components {
                collect_component(loader, component, &transform, out, stack)?;
            }
            stack.pop();
            Ok(())
        }
    }
}

fn collect_component(
    loader: &GlyfLazyLoader<'_>,
    component: &CompositeComponent,
    parent: &Transform,
    out: &mut Vec<Vec<Point>>,
    stack: &mut Vec<u16>,
) -> Result<(), FontError> {
    let (inner, anchors) = parse_component(component)?;
    let combined = parent.combine(&inner);

    if let Some((parent_index, component_index)) = anchors {
        // 点匹配：组件自身的平移为零，展开后再按锚点差值在父坐标系里对齐
        let mut sub = Vec::new();
        collect(loader, component.glyph_index, combined, &mut sub, stack)?;

        let parent_point = out.iter().flatten().nth(parent_index as usize).copied();
        let component_point = sub.iter().flatten().nth(component_index as usize).copied();
        if let (Some(parent_point), Some(component_point)) = (parent_point, component_point) {
            let dx = parent_point.x - component_point.x;
            let dy = parent_point.y - component_point.y;
            for point in sub.iter_mut().flatten() {
                point.x += dx;
                point.y += dy;
            }
        }
        out.extend(sub);
        Ok(())
    } else {
        collect(loader, component.glyph_index, combined, out, stack)
    }
}

/// 解析组件的平移与变换矩阵，返回锚点索引（点匹配模式）
fn parse_component(
    component: &CompositeComponent,
) -> Result<(Transform, Option<(u16, u16)>), FontError> {
    let mut reader = Reader::new(&component.data);
    let words = component.flags & ARG_1_AND_2_ARE_WORDS != 0;
    let xy_values = component.flags & ARGS_ARE_XY_VALUES != 0;

    let (dx, dy, anchors) = if xy_values {
        let (dx, dy) = if words {
            (reader.read_i16()? as f64, reader.read_i16()? as f64)
        } else {
            (
                reader.read_u8()? as i8 as f64,
                reader.read_u8()? as i8 as f64,
            )
        };
        (dx, dy, None)
    } else {
        let (first, second) = if words {
            (reader.read_u16()?, reader.read_u16()?)
        } else {
            (reader.read_u8()? as u16, reader.read_u8()? as u16)
        };
        (0.0, 0.0, Some((first, second)))
    };

    let read_f2dot14 = |reader: &mut Reader<'_>| -> Result<f64, FontError> {
        Ok(reader.read_i16()? as f64 / F2DOT14_SCALE)
    };

    let (a, b, c, d) = if component.flags & WE_HAVE_A_SCALE != 0 {
        let scale = read_f2dot14(&mut reader)?;
        (scale, 0.0, 0.0, scale)
    } else if component.flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
        let x_scale = read_f2dot14(&mut reader)?;
        let y_scale = read_f2dot14(&mut reader)?;
        (x_scale, 0.0, 0.0, y_scale)
    } else if component.flags & WE_HAVE_A_TWO_BY_TWO != 0 {
        let a = read_f2dot14(&mut reader)?;
        let b = read_f2dot14(&mut reader)?;
        let c = read_f2dot14(&mut reader)?;
        let d = read_f2dot14(&mut reader)?;
        (a, b, c, d)
    } else {
        (1.0, 0.0, 0.0, 1.0)
    };

    Ok((
        Transform {
            a,
            b,
            c,
            d,
            e: dx,
            f: dy,
        },
        anchors,
    ))
}

fn push_simple(simple: &SimpleGlyph, transform: &Transform, out: &mut Vec<Vec<Point>>) {
    let total_points = simple
        .x_coordinates
        .len()
        .min(simple.y_coordinates.len())
        .min(simple.flags.len());

    let mut start = 0usize;
    for &end in &simple.end_pts_of_contours {
        let end = (end as usize).min(total_points.saturating_sub(1));
        if start > end {
            continue;
        }

        let mut contour = Vec::with_capacity(end - start + 1);
        for index in start..=end {
            let (x, y) = transform.apply(
                simple.x_coordinates[index] as f64,
                simple.y_coordinates[index] as f64,
            );
            contour.push(Point {
                x,
                y,
                on_curve: simple.flags[index] & FLAG_ON_CURVE != 0,
            });
        }
        out.push(contour);
        start = end + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn font() -> Font {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rfont/src/AlimamaDaoLiTi.ttf");
        Font::load(path).unwrap()
    }

    #[test]
    fn outline_of_latin_glyph_has_contours() {
        let font = font();
        let glyph_id = font.text_to_glyph_ids("A")[0];
        let outline = glyph_outline(&font, glyph_id).unwrap();

        assert!(!outline.contours.is_empty());
        assert!(outline.advance_width > 0);

        let (min_x, min_y, max_x, max_y) = outline.bounds().unwrap();
        assert!(min_x < max_x);
        assert!(min_y < max_y);
        for point in outline.contours.iter().flatten() {
            assert!((min_x..=max_x).contains(&point.x));
            assert!((min_y..=max_y).contains(&point.y));
        }
    }

    #[test]
    fn composite_glyph_is_expanded_with_component_offsets() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/composite.ttf");
        let font = Font::load(path).unwrap();

        let outline_of = |ch: &str| glyph_outline(&font, font.text_to_glyph_ids(ch)[0]).unwrap();
        let simple = outline_of("A");
        let composite = outline_of("B");

        // 夹具中的 B 由两个 A 组成，偏移分别为 (100, 0) 与 (0, 200)
        assert_eq!(composite.contours.len(), simple.contours.len() * 2);
        assert_bounds(&composite, (0.0, 0.0, 600.0, 700.0));

        // C 为 2 倍缩放（F2Dot14 精度，允许极小误差）
        assert_bounds(&outline_of("C"), (0.0, 0.0, 1000.0, 1000.0));

        // D 为 90 度旋转，验证矩阵分量顺序
        assert_bounds(&outline_of("D"), (-500.0, 0.0, 0.0, 500.0));
    }

    fn assert_bounds(outline: &GlyphOutline, expected: (f64, f64, f64, f64)) {
        let actual = outline.bounds().unwrap();
        let values = [actual.0, actual.1, actual.2, actual.3];
        let expected = [expected.0, expected.1, expected.2, expected.3];
        for (value, want) in values.iter().zip(expected) {
            assert!(
                (value - want).abs() < 0.1,
                "包围盒不匹配：{values:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn transform_combine_applies_inner_first() {
        let outer = Transform {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: 10.0,
            f: 0.0,
        };
        let inner = Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 3.0,
            f: 4.0,
        };

        let (x, y) = outer.combine(&inner).apply(0.0, 0.0);
        assert_eq!((x, y), (16.0, 8.0));
    }
}
