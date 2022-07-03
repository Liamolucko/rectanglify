//! A binary which takes an image and rectanglifies it.

use anyhow::{anyhow, Context};
use image::GrayImage;
use rects::{rectanglify, Settings};
use std::env;

mod rects;

fn main() -> anyhow::Result<()> {
    let [_, in_path, out_path]: [_; 3] = env::args_os()
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|vec: Vec<_>| anyhow!("expected 2 arguments, got {}", vec.len() - 1))?;

    let input = image::open(&in_path)
        .with_context(|| format!("failed to open {}", in_path.to_string_lossy()))?;
    let mut output = GrayImage::new(input.width(), input.height());

    rectanglify(&input, &mut output, Settings::default());

    output.save(out_path).context("failed to save output")?;

    Ok(())
}
