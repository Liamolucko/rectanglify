//! The actual rectangle drawing algorithm, which can work with any image integrated with the `image ` crate.

use image::{GenericImage, GenericImageView, Pixel, Primitive};
use num_traits::ToPrimitive;

pub const DEFAULT_RECTS_PER_PIXEL: f32 = 0.01;

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub rects_per_pixel: f32,
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
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Rectangle {
    fn width(&self) -> f32 {
        self.right - self.left
    }

    fn height(&self) -> f32 {
        self.bottom - self.top
    }
}

fn darkness<P: Pixel>(p: P) -> f32 {
    1.0 - p.to_luma()[0].to_f32().unwrap() / P::Subpixel::DEFAULT_MAX_VALUE.to_f32().unwrap()
}

fn darkness_at(image: &impl GenericImageView, rect: Rectangle, x: u32, y: u32) -> f32 {
    if image.in_bounds(x, y) {
        let mut darkness = darkness(image.get_pixel(x, y));

        if rect.left as u32 == x {
            darkness *= 1.0 - rect.left % 1.0;
        } else if rect.right as u32 == x {
            darkness *= rect.right % 1.0;
        }

        if rect.top as u32 == y {
            darkness *= 1.0 - rect.top % 1.0;
        } else if rect.bottom as u32 == y {
            darkness *= rect.bottom % 1.0;
        }

        darkness
    } else {
        0.0
    }
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
        if image.in_bounds(x, y) {
            image.put_pixel(x, y, black);
        }
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
        if image.in_bounds(x, y) {
            image.put_pixel(x, y, black);
        }
    }
}

pub fn rectanglify<I: GenericImageView, O: GenericImage>(
    input: &I,
    output: &mut O,
    settings: Settings,
) {
    let total_darkness: f32 = input.pixels().map(|(_, _, p)| darkness(p)).sum();
    let num_rects = (total_darkness * settings.rects_per_pixel).round() as usize;

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
            right: input.width() as f32,
            bottom: input.height() as f32,
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

    if area.width() > area.height() {
        // split it horizontally
        let mut darkness = 0.0;
        for x in area.left as u32..area.right as u32 {
            for y in area.top as u32..area.bottom as u32 {
                darkness += darkness_at(input, area, x, y);
                if darkness * settings.rects_per_pixel >= (rects / 2) as f32 {
                    // We found the split! draw a line
                    vertical_line(output, x, area.top as u32, area.bottom as u32);
                    // then recurse
                    let split = x as f32 + (y - area.top as u32) as f32 / area.height() as f32;

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
        }
    } else {
        // split it vertically
        let mut darkness = 0.0;
        for y in area.top as u32..area.bottom as u32 {
            for x in area.left as u32..area.right as u32 {
                darkness += darkness_at(input, area, x, y);
                if darkness * settings.rects_per_pixel >= (rects / 2) as f32 {
                    // We found the split! draw a line
                    horizontal_line(output, y, area.left as u32, area.right as u32);
                    // then recurse
                    let split = y as f32 + (x - area.left as u32) as f32 / area.width() as f32;

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
}
