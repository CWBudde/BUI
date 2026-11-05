//! Which rows and columns are on screen: the one place this math lives.
//!
//! Blender's spreadsheet draws a fixed-height row table by computing the
//! visible rows from the scroll offset (`get_visible_rows` in
//! `spreadsheet_draw.cc`: `first_row = scroll_y / row_height`,
//! `max_visible_rows = region_h / row_height + 1`) and skipping columns that
//! are off the region (`draw_spreadsheet_cells`: draw when
//! `right_x >= left_edge && left_x <= region_w`). Cost per frame is
//! O(visible cells), independent of the row count. [`uniform_list`] and
//! [`DataTable`] both use these functions.
//!
//! All math is `f64` with saturating casts, so a billion rows or a NaN
//! offset can neither overflow nor panic.
//!
//! [`uniform_list`]: crate::uniform_list
//! [`DataTable`]: crate::table::DataTable

use std::ops::Range;

/// The rows (`first..end`) a region `view_h` tall shows, scrolled `scroll_y`
/// px down, when every row is `row_h` px tall: Blender's `get_visible_rows`
/// made exact for fractional scrolls (the row cut at the bottom edge counts)
/// and clamped to `tot_rows`.
#[must_use]
pub fn visible_rows(scroll_y: f64, row_h: f64, view_h: f64, tot_rows: usize) -> Range<usize> {
    if tot_rows == 0 || row_h.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || view_h.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return 0..0;
    }
    let top = if scroll_y > 0.0 { scroll_y } else { 0.0 };
    let first = ((top / row_h) as usize).min(tot_rows);
    let end = (((top + view_h) / row_h).ceil() as usize).min(tot_rows);
    first..end.max(first)
}

/// The largest scroll offset: the content ends at the region's bottom.
#[must_use]
pub fn max_scroll(tot: usize, item: f64, view: f64) -> f64 {
    let m = tot as f64 * item - view;
    if m > 0.0 { m } else { 0.0 }
}

/// `scroll` limited to `0..=max_scroll` (NaN becomes 0).
#[must_use]
pub fn clamp_scroll(scroll: f64, tot: usize, item: f64, view: f64) -> f64 {
    if scroll.is_nan() { 0.0 } else { scroll.clamp(0.0, max_scroll(tot, item, view)) }
}

/// The scroll offset that brings item `ix` fully into view from `scroll`
/// (unchanged when it already is).
#[must_use]
pub fn scroll_to_show(scroll: f64, ix: usize, item: f64, view: f64) -> f64 {
    let top = ix as f64 * item;
    let bottom = top + item;
    if top < scroll {
        top
    } else if bottom > scroll + view {
        bottom - view
    } else {
        scroll
    }
}

/// Columns of a strip of consecutive columns (`x[i]` = left edge of column
/// `i`, `x[n]` = the right edge of the last: `n + 1` entries) that
/// intersect `lo..hi` (view coordinates, already shifted by the scroll):
/// Blender's column culling, in `O(log n + visible)` by bisection.
#[must_use]
pub fn visible_cols(x: &[f32], lo: f32, hi: f32) -> Range<usize> {
    let n = x.len().saturating_sub(1);
    if n == 0 || hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
        return 0..0;
    }
    // first column whose right edge reaches `lo`; first whose left edge passes `hi`
    let first = x[1..].partition_point(|&r| r < lo).min(n);
    let end = x[..n].partition_point(|&l| l <= hi);
    first..end.max(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_degenerate() {
        assert_eq!(visible_rows(0.0, 20.0, 100.0, 0), 0..0);
        assert_eq!(visible_rows(0.0, 0.0, 100.0, 10), 0..0);
        assert_eq!(visible_rows(0.0, -1.0, 100.0, 10), 0..0);
        assert_eq!(visible_rows(0.0, 20.0, 0.0, 10), 0..0);
        assert_eq!(visible_rows(f64::NAN, 20.0, 100.0, 10), 0..5);
        assert_eq!(visible_rows(0.0, f64::NAN, 100.0, 10), 0..0);
        assert_eq!(visible_rows(0.0, 20.0, f64::NAN, 10), 0..0);
    }

    #[test]
    fn fewer_rows_than_viewport() {
        assert_eq!(visible_rows(0.0, 20.0, 1000.0, 3), 0..3);
    }

    #[test]
    fn fractional_scroll_counts_the_cut_row() {
        // rows 0.5 .. 5.5 of 20px rows: six rows touch the region
        assert_eq!(visible_rows(10.0, 20.0, 100.0, 100), 0..6);
        assert_eq!(visible_rows(40.0, 20.0, 100.0, 100), 2..7);
        for s in 0..400 {
            let r = visible_rows(f64::from(s) * 0.37, 20.0, 103.0, 1000);
            assert!(r.len() <= (103 / 20) + 2);
        }
    }

    #[test]
    fn scroll_past_end_and_huge() {
        assert_eq!(visible_rows(1e9, 20.0, 100.0, 10), 10..10);
        assert_eq!(visible_rows(f64::INFINITY, 20.0, 100.0, 10), 10..10);
        assert_eq!(visible_rows(-50.0, 20.0, 100.0, 10), 0..5);
        let r = visible_rows(1e15, 24.0, 600.0, usize::MAX);
        assert!(r.start < r.end && r.len() <= 27);
        assert_eq!(clamp_scroll(1e30, 10, 20.0, 100.0), 100.0);
        assert_eq!(clamp_scroll(f64::NAN, 10, 20.0, 100.0), 0.0);
        assert_eq!(max_scroll(5, 20.0, 1000.0), 0.0);
        // past i32::MAX pixels: 100M rows * 24 px
        let r = visible_rows(2.4e9 - 48.0, 24.0, 48.0, 100_000_000);
        assert_eq!(r, 99_999_998..100_000_000);
    }

    #[test]
    fn resize_keeps_first_row() {
        let a = visible_rows(200.0, 20.0, 100.0, 100);
        let b = visible_rows(200.0, 20.0, 300.0, 100);
        assert_eq!(a.start, b.start);
        assert!(b.len() > a.len());
    }

    #[test]
    fn show_item() {
        assert_eq!(scroll_to_show(0.0, 10, 20.0, 100.0), 120.0);
        assert_eq!(scroll_to_show(300.0, 2, 20.0, 100.0), 40.0);
        assert_eq!(scroll_to_show(40.0, 3, 20.0, 100.0), 40.0);
    }

    #[test]
    fn culling_matches_naive() {
        let widths = [30.0f32, 50.0, 20.0, 80.0, 10.0, 60.0];
        let mut x = vec![0.0f32];
        for w in widths {
            x.push(x.last().unwrap() + w);
        }
        for lo in (-20..260).step_by(7) {
            for len in [0, 1, 40, 90, 400] {
                let (lo, hi) = (lo as f32, (lo + len) as f32);
                let naive: Vec<usize> = (0..widths.len()).filter(|&i| x[i + 1] >= lo && x[i] <= hi).collect();
                let r = visible_cols(&x, lo, hi);
                if hi > lo {
                    assert_eq!(r.collect::<Vec<_>>(), naive, "lo {lo} hi {hi}");
                } else {
                    assert!(r.is_empty());
                }
            }
        }
        assert_eq!(visible_cols(&[], 0.0, 10.0), 0..0);
        assert_eq!(visible_cols(&[0.0], 0.0, 10.0), 0..0);
    }
}
