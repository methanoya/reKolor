mod info;
mod utils;
mod conv;
mod console;

use std::fs;
// use std::io::Read;
use std::io::Write;
use conv::{RgbColor8, RgbColorReplacementPair};

fn main() {
    println!("Hello, main is here!");
    let file_path_input = "../typescript/samples/icon-calendar.png";
    let file_path_output = "../typescript/samples/output-icon-calendar.png";
    println!("Going to read {}", file_path_input);
    let raw = fs::read(file_path_input).expect("Couldn't find file");
    println!("{:?}", info::image_info(&raw));
    let colors: Vec<RgbColorReplacementPair> = vec![
        RgbColorReplacementPair {
            // rgb (230, 75, 59), alpha(0.996) #e64c3c
            from: RgbColor8 {
                r: 230u8,
                g: 76u8,
                b: 60u8,
            },
            // Pantone 179
            to: RgbColor8 {
                r: 226u8, g: 61u8, b: 40u8,
            },
        },
        RgbColorReplacementPair {
            // rgb (235, 239, 240), alpha(0.996) #ebeff0
            from: RgbColor8 {
                r: 235u8,
                g: 239u8,
                b: 240u8,
            },
            // Pantone 656
            to: RgbColor8 {
                r: 214u8,
                g: 219u8,
                b: 224u8,
            },
        },
        RgbColorReplacementPair {
            // rgb (0, 0, 0), alpha(0.000) #ffffff
            from: RgbColor8 {
                r: 255u8,
                g: 255u8,
                b: 255u8,
            },
            // Pure White (non-palette)
            to: RgbColor8 {
                r: 255u8,
                g: 255u8,
                b: 255u8,
            },
        }];
    let result = conv::replace_rgb_colors(&raw, &colors).unwrap();
    println!("result: {} x {}", result.width, result.height);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(file_path_output)
        .unwrap();

    println!("Output file: {:?}", file);
    println!("{:?}", file.write(&result.image[..]));
}