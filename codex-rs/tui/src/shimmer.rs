use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::color::blend;
use crate::terminal_palette::default_bg;
use crate::terminal_palette::default_fg;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
const SHIMMER_PADDING: usize = 10;
const SHIMMER_SWEEP_SECONDS: f64 = 2.0;

fn elapsed_since_start() -> Duration {
    let start = PROCESS_START.get_or_init(Instant::now);
    start.elapsed()
}

fn shimmer_period(text: &str) -> Option<usize> {
    let char_count = text.chars().count();
    if char_count == 0 {
        None
    } else {
        Some(char_count + SHIMMER_PADDING * 2)
    }
}

pub(crate) fn next_shimmer_frame_delay(text: &str) -> Option<Duration> {
    let period = shimmer_period(text)?;
    let step_seconds = SHIMMER_SWEEP_SECONDS / period as f64;
    let phase_seconds = elapsed_since_start().as_secs_f64() % SHIMMER_SWEEP_SECONDS;
    let next_step = (phase_seconds / step_seconds).floor() + 1.0;
    let next_phase_seconds = (next_step * step_seconds).min(SHIMMER_SWEEP_SECONDS);
    let delay_seconds = (next_phase_seconds - phase_seconds).max(step_seconds);
    Some(Duration::from_secs_f64(delay_seconds).max(Duration::from_millis(1)))
}

pub(crate) fn shimmer_spans(text: &str) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    // Use time-based sweep synchronized to process start.
    let period = chars.len() + SHIMMER_PADDING * 2;
    let pos_f = (elapsed_since_start().as_secs_f32() % SHIMMER_SWEEP_SECONDS as f32)
        / SHIMMER_SWEEP_SECONDS as f32
        * (period as f32);
    let pos = pos_f as usize;
    let has_true_color = supports_color::on_cached(supports_color::Stream::Stdout)
        .map(|level| level.has_16m)
        .unwrap_or(false);
    let band_half_width = 5.0;

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(chars.len());
    let base_color = default_fg().unwrap_or((128, 128, 128));
    let highlight_color = default_bg().unwrap_or((255, 255, 255));
    for (i, ch) in chars.iter().enumerate() {
        let i_pos = i as isize + SHIMMER_PADDING as isize;
        let pos = pos as isize;
        let dist = (i_pos - pos).abs() as f32;

        let t = if dist <= band_half_width {
            let x = std::f32::consts::PI * (dist / band_half_width);
            0.5 * (1.0 + x.cos())
        } else {
            0.0
        };
        let style = if has_true_color {
            let highlight = t.clamp(0.0, 1.0);
            let (r, g, b) = blend(highlight_color, base_color, highlight * 0.9);
            // Allow custom RGB colors, as the implementation is thoughtfully
            // adjusting the level of the default foreground color.
            #[allow(clippy::disallowed_methods)]
            {
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            color_for_level(t)
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    spans
}

fn color_for_level(intensity: f32) -> Style {
    // Tune fallback styling so the shimmer band reads even without RGB support.
    if intensity < 0.2 {
        Style::default().add_modifier(Modifier::DIM)
    } else if intensity < 0.6 {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    }
}
