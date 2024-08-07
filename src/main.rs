use anyhow::{anyhow, Result};
use clap::{command, Parser};
use hanbun::{Cell, Color};
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    images: Vec<PathBuf>,

    #[arg(short, long)]
    #[arg(value_parser = clap::value_parser!(f32))]
    /// Scale image by value, values below 1.0 shrink the image
    scale: Option<f32>,

    #[arg(short, long)]
    #[arg(value_parser = parse_padding)]
    /// Add padding around the image
    ///
    /// Can be one value for all sides or up to
    /// 4 values following CSS padding rules
    padding: Option<(u32, u32, u32, u32)>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (images, errors): (Vec<_>, Vec<_>) = cli
        .images
        .iter()
        .map(|v| get_image(v.to_str()))
        .partition(Result::is_ok);

    let images: Vec<_> = images.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();

    for mut image in images {
        if let Some(scale_factor) = cli.scale {
            image = scale_image(image, scale_factor);
        }

        draw(image, cli.padding)?;
        println!("\n");
    }

    for error in errors {
        println!("{}", error);
    }

    Ok(())
}

fn parse_padding(padding: &str) -> Result<(u32, u32, u32, u32), String> {
    let parts: Vec<_> = padding.split_whitespace().collect();

    let values: Result<Vec<_>, _> = parts.iter().map(|&part| u32::from_str(part)).collect();
    let values = values.map_err(|_| format!("Invalid input: {}", padding))?;

    match values.len() {
        1 => {
            let v = values[0];
            Ok((v, v, v, v))
        }
        2 => {
            let (vert, horiz) = (values[0], values[1]);
            Ok((vert, horiz, vert, horiz))
        }
        3 => {
            let (top, horiz, bot) = (values[0], values[1], values[2]);
            Ok((top, horiz, bot, horiz))
        }
        4 => {
            let (top, right, bot, left) = (values[0], values[1], values[2], values[3]);
            Ok((top, right, bot, left))
        }
        _ => Err("Invalid number of values".into()),
    }
}

fn get_image(path: Option<&str>) -> Result<DynamicImage> {
    let Some(file) = path else {
        return Err(anyhow!("Invalid characters in path"));
    };
    ImageReader::open(file)
        .map_err(|_| anyhow!("Invalid image path: {}", file))?
        .decode()
        .map_err(|_| anyhow!("Failed to decode image: {}", file))
}

fn scale_image(image: DynamicImage, scale: f32) -> DynamicImage {
    let nwidth = image.width() as f32 * scale;
    let nheight = image.height() as f32 * scale;

    image.resize(nwidth as u32, nheight as u32, FilterType::CatmullRom)
}

fn draw(img: DynamicImage, padding: Option<(u32, u32, u32, u32)>) -> Result<()> {
    let (mut width, mut height) = (img.width(), img.height());
    if let Some((top, right, bottom, left)) = padding {
        width += left + right;
        height += top + bottom;
    }
    let mut buffer = hanbun::Buffer::new(width as usize, height as usize / 2, ' ');

    let rgb = img.into_rgba8();

    for y in 0..height / 2 {
        for x in 0..width {
            let (top_pixel, bot_pixel) = match padding {
                Some((top, right, bottom, left)) => {
                    if y * 2 < top || y * 2 >= height - bottom || x < left || x >= width - right {
                        (None, None)
                    } else {
                        (
                            rgb.get_pixel_checked(x - left, y * 2 - top),
                            rgb.get_pixel_checked(x - left, (y * 2) + 1 - top),
                        )
                    }
                }
                None => (
                    rgb.get_pixel_checked(x, y * 2),
                    rgb.get_pixel_checked(x, (y * 2) + 1),
                ),
            };

            buffer.cells[((y * width) + x) as usize] = Cell {
                char: Some(' '),
                char_color: None,
                upper_block: pixel_to_cell_color(top_pixel),
                lower_block: pixel_to_cell_color(bot_pixel),
            };
        }
    }

    buffer.draw();

    Ok(())
}

fn pixel_to_cell_color(pixel_opt: Option<&image::Rgba<u8>>) -> Option<Option<Color>> {
    pixel_opt.and_then(|p| {
        let alpha = p.0[3];
        if alpha == 255 {
            Some(Some(Color::Rgb {
                r: p.0[0],
                g: p.0[1],
                b: p.0[2],
            }))
        } else {
            None
        }
    })
}
