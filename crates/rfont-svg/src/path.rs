//! 轮廓到 SVG path 数据的转换

use crate::outline::{GlyphOutline, Point};

/// 将字形轮廓转换为 SVG `path` 的 `d` 属性
pub fn outline_to_path(outline: &GlyphOutline) -> String {
    contours_to_path(&outline.contours)
}

/// 将轮廓集合转换为 SVG `path` 的 `d` 属性
///
/// TrueType 的曲线是二次贝塞尔：控制点之间若没有落在曲线上的点，
/// 就以下一段的隐含中点作为该段终点。
pub fn contours_to_path(contours: &[Vec<Point>]) -> String {
    let mut path = String::new();
    for contour in contours {
        append_contour(contour, &mut path);
    }
    path
}

fn append_contour(points: &[Point], out: &mut String) {
    let count = points.len();
    if count == 0 {
        return;
    }

    // 起点必须是曲线上的点；首尾都是控制点时，取两者中点作为隐含起点
    let (start, first_index, steps) = if points[0].on_curve {
        (points[0], 1, count - 1)
    } else if points[count - 1].on_curve {
        (points[count - 1], 0, count - 1)
    } else {
        (midpoint(points[count - 1], points[0]), 0, count)
    };

    out.push_str(&format!("M{} {}", num(start.x), num(start.y)));

    let mut control: Option<Point> = None;
    for step in 0..steps {
        let point = points[(first_index + step) % count];
        if point.on_curve {
            match control.take() {
                Some(control) => push_quad(out, control, point),
                None => push_line(out, point),
            }
        } else {
            if let Some(previous) = control.take() {
                let implied = midpoint(previous, point);
                push_quad(out, previous, implied);
            }
            control = Some(point);
        }
    }

    // 结尾残留的控制点需要连回起点
    if let Some(control) = control {
        push_quad(out, control, start);
    }

    out.push('Z');
}

fn push_line(out: &mut String, point: Point) {
    out.push_str(&format!("L{} {}", num(point.x), num(point.y)));
}

fn push_quad(out: &mut String, control: Point, end: Point) {
    out.push_str(&format!(
        "Q{} {} {} {}",
        num(control.x),
        num(control.y),
        num(end.x),
        num(end.y)
    ));
}

fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
        on_curve: true,
    }
}

/// 格式化坐标：最多保留 3 位小数，去掉多余的 0
pub(crate) fn num(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        return "0".to_string();
    }
    let text = format!("{rounded:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, on_curve: bool) -> Point {
        Point { x, y, on_curve }
    }

    #[test]
    fn square_contour_is_all_lines() {
        let contour = vec![
            point(0.0, 0.0, true),
            point(100.0, 0.0, true),
            point(100.0, 100.0, true),
            point(0.0, 100.0, true),
        ];
        assert_eq!(contours_to_path(&[contour]), "M0 0L100 0L100 100L0 100Z");
    }

    #[test]
    fn off_curve_control_produces_quadratic_segment() {
        let contour = vec![
            point(0.0, 0.0, true),
            point(50.0, 100.0, false),
            point(100.0, 0.0, true),
        ];
        assert_eq!(contours_to_path(&[contour]), "M0 0Q50 100 100 0Z");
    }

    #[test]
    fn consecutive_controls_use_implied_midpoints() {
        let contour = vec![
            point(0.0, 0.0, true),
            point(0.0, 100.0, false),
            point(100.0, 100.0, false),
            point(100.0, 0.0, true),
        ];
        assert_eq!(
            contours_to_path(&[contour]),
            "M0 0Q0 100 50 100Q100 100 100 0Z"
        );
    }

    #[test]
    fn contour_starting_with_control_point_closes_to_start() {
        let contour = vec![
            point(0.0, 100.0, false),
            point(100.0, 100.0, false),
            point(100.0, 0.0, true),
            point(0.0, 0.0, true),
        ];
        // 末点在曲线上时以它作为起点，开头的控制点由二次曲线接到隐含中点
        let path = contours_to_path(&[contour]);
        assert_eq!(path, "M0 0Q0 100 50 100Q100 100 100 0Z");
    }

    #[test]
    fn number_formatting_trims_noise() {
        assert_eq!(num(12.5), "12.5");
        assert_eq!(num(12.0), "12");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(1.23456), "1.235");
    }
}
