//! The actual rectangle drawing algorithm, which can work with any image integrated with the `image ` crate.

use image::{GenericImage, GenericImageView, Pixel, Primitive};
use num_traits::ToPrimitive;

pub const DEFAULT_RECTS_PER_PIXEL: f64 = 0.1;

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub rects_per_pixel: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            rects_per_pixel: DEFAULT_RECTS_PER_PIXEL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rectangle {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Rectangle {
    fn width(&self) -> f64 {
        self.right - self.left
    }

    fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

fn darkness<P: Pixel>(p: P) -> f64 {
    1.0 - p.to_luma()[0].to_f64().unwrap() / P::Subpixel::DEFAULT_MAX_VALUE.to_f64().unwrap()
}

fn darkness_at(image: &impl GenericImageView, rect: Rectangle, x: u32, y: u32) -> f64 {
    let mut darkness = darkness(image.get_pixel(x, y));

    if (x as f64) < rect.left {
        darkness *= f64::max((x + 1) as f64 - rect.left, 0.0);
    } else if (x + 1) as f64 > rect.right {
        darkness *= f64::max(rect.right - x as f64, 0.0);
    }

    if (y as f64) < rect.top {
        darkness *= f64::max((y + 1) as f64 - rect.top, 0.0);
    } else if (y + 1) as f64 > rect.bottom {
        darkness *= f64::max(rect.bottom - y as f64, 0.0);
    }

    darkness
}

fn horizontal_line<I: GenericImage>(image: &mut I, y: u32, start_x: u32, end_x: u32) {
    let black = <I::Pixel as Pixel>::from_slice(&vec![
            // make everything 0 for black
            <I::Pixel as Pixel>::Subpixel::DEFAULT_MIN_VALUE;
            <I::Pixel as Pixel>::CHANNEL_COUNT as usize
        ])
    // except alpha, which should be maxed
    .map_with_alpha(|x| x, |_| <I::Pixel as Pixel>::Subpixel::DEFAULT_MAX_VALUE);

    for x in start_x..=end_x {
        image.put_pixel(x, y, black);
    }
}

fn vertical_line<I: GenericImage>(image: &mut I, x: u32, start_y: u32, end_y: u32) {
    let black = <I::Pixel as Pixel>::from_slice(&vec![
            // make everything 0 for black
            <I::Pixel as Pixel>::Subpixel::DEFAULT_MIN_VALUE;
            <I::Pixel as Pixel>::CHANNEL_COUNT as usize
        ])
    // except alpha, which should be maxed
    .map_with_alpha(|x| x, |_| <I::Pixel as Pixel>::Subpixel::DEFAULT_MAX_VALUE);

    for y in start_y..=end_y {
        image.put_pixel(x, y, black);
    }
}

pub fn rectanglify<I: GenericImageView, O: GenericImage>(
    input: &I,
    output: &mut O,
    mut settings: Settings,
) {
    let total_darkness: f64 = input.pixels().map(|(_, _, p)| darkness(p)).sum();
    let num_rects = (total_darkness * settings.rects_per_pixel).round() as usize;
    // Adjust this so that it actually matches the number of rectangles we're drawing.
    settings.rects_per_pixel = num_rects as f64 / total_darkness;

    // fill the output with white to start with
    let white = *<O::Pixel as Pixel>::from_slice(&vec![
        // make everything max for black
        <O::Pixel as Pixel>::Subpixel::DEFAULT_MAX_VALUE;
        <O::Pixel as Pixel>::CHANNEL_COUNT as usize
    ]);
    for x in 0..output.width() {
        for y in 0..output.height() {
            output.put_pixel(x, y, white)
        }
    }

    draw_rects(
        input,
        output,
        settings,
        Rectangle {
            left: 0.0,
            top: 0.0,
            right: input.width() as f64,
            bottom: input.height() as f64,
        },
        num_rects,
    )
}

fn draw_rects(
    input: &impl GenericImageView,
    output: &mut impl GenericImage,
    settings: Settings,
    area: Rectangle,
    rects: usize,
) {
    if rects == 1 {
        return;
    }

    // The amount of darkness we've found so far.
    let mut darkness = 0.0;

    // The target number of rectangles to be in the first half.
    let target_rects = rects / 2;
    // The target amount of darkness in the first half.
    let target_darkness = target_rects as f64 / settings.rects_per_pixel;

    if area.width() > area.height() {
        // split it horizontally
        for x in area.left.floor() as u32..area.right.ceil() as u32 {
            let mut column_darkness = 0.0;
            for y in area.top.floor() as u32..area.bottom.ceil() as u32 {
                column_darkness += darkness_at(input, area, x, y);
            }
            darkness += column_darkness;

            if darkness >= target_darkness {
                // We found the split! draw a line
                vertical_line(
                    output,
                    x,
                    area.top.floor() as u32,
                    area.bottom.ceil() as u32 - 1,
                );

                let overshoot = darkness - target_darkness;
                // Find the exact point of the split by taking away the amount we overshot.
                let split = (x + 1) as f64 - overshoot / column_darkness;

                let left = Rectangle {
                    right: split,
                    ..area
                };
                let right = Rectangle {
                    left: split,
                    ..area
                };

                draw_rects(input, output, settings, left, rects / 2);
                draw_rects(input, output, settings, right, rects - rects / 2);

                return;
            }
        }
    } else {
        // split it vertically
        for y in area.top.floor() as u32..area.bottom.ceil() as u32 {
            let mut row_darkness = 0.0;
            for x in area.left.floor() as u32..area.right.ceil() as u32 {
                row_darkness += darkness_at(input, area, x, y);
            }
            darkness += row_darkness;

            if darkness >= target_darkness {
                // We found the split! draw a line
                horizontal_line(
                    output,
                    y,
                    area.left.floor() as u32,
                    area.right.ceil() as u32 - 1,
                );

                let overshoot = darkness - target_darkness;
                // Find the exact point of the split by taking away the amount we overshot.
                let split = (y + 1) as f64 - overshoot / row_darkness;

                let top = Rectangle {
                    bottom: split,
                    ..area
                };
                let bottom = Rectangle { top: split, ..area };

                draw_rects(input, output, settings, top, rects / 2);
                draw_rects(input, output, settings, bottom, rects - rects / 2);

                return;
            }
        }
    }
}
