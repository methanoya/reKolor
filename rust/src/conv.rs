use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use wasm_bindgen::JsValue;
use std::io::Cursor;
use image::{GenericImageView, ImageBuffer, Pixel, Rgb, Rgba};
use deltae::{DE2000, LabValue, Delta};
use lab::Lab;
use crate::utils;
use crate::console::log_1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RgbColor<T> {
    pub r: T,
    pub g: T,
    pub b: T,
}

impl<T: for<'a> Deserialize<'a>> From<JsValue> for RgbColor<T> {
    fn from(value: JsValue) -> Self {
        serde_wasm_bindgen::from_value(value).unwrap()
    }
}
#[typeshare]
pub type RgbColor8 = RgbColor<u8>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColorTranslation<From, To = From> {
    pub from: From,
    pub to: To,
}

impl<T: for<'a> Deserialize<'a>> From<JsValue> for ColorTranslation<T> {
    fn from(value: JsValue) -> Self {
        match serde_wasm_bindgen::from_value(value) {
            Ok(value) => value,
            Err(err) => {
                log_1(&format!("Conversion error: {:?}", err).into());
                unreachable!();
            },
        }
    }
}


#[typeshare]
pub type RgbColorReplacementPair = ColorTranslation<RgbColor8>;

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConvResult {
    pub image: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Into<JsValue> for ConvResult {
    fn into(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self).unwrap()
    }
}
type RgbColorTranslation = ColorTranslation<RgbColor8, Rgb<u8>>;
struct TranslationCache {
    rgb: RgbColorTranslation,
    lab: LabValue,
}

impl From<&RgbColorReplacementPair> for TranslationCache {
    fn from(value: &RgbColorReplacementPair) -> Self {
        let RgbColor8 {r, g, b} = value.to;
        Self {
            rgb: RgbColorTranslation {
                from: value.from.clone(),
                to: [r, g, b].into(),
            },
            lab: (&value.to).into(),
        }
    }
}

impl From<&RgbColor8> for LabValue {
    fn from(value: &RgbColor8) -> Self {
        let rgb: [u8; 3] = [value.r, value.g, value.b];
        let Lab { l, a, b } = Lab::from_rgb(&rgb);
        Self {
            l,
            a,
            b,
        }
    }
}

fn rgba2rgb(input: Rgba<u8>) -> Rgb<u8> {
    match input.channels() {
        [r, g, b, a] => {
            let factor: u8 = 255u8.checked_sub(*a).unwrap();
            Rgb([
                factor.checked_add((((*a as usize) * (*r as usize)) / 255usize) as u8).unwrap(),
                factor.checked_add((((*a as usize) * (*g as usize)) / 255usize) as u8).unwrap(),
                factor.checked_add((((*a as usize) * (*b as usize)) / 255usize) as u8).unwrap(),
            ])
        },
        _ => unreachable!("{:?}", input),
    }
}

fn lookup(map: &Vec<TranslationCache>, pixel:  &Rgb<u8>) -> (Rgb<u8>, bool) {
    match pixel.channels() {
        [pixel_r, pixel_g, pixel_b] => {
            let mut best_distance = f32::MAX;
            let mut best_color = pixel;
            for entry in map {
                let RgbColor8 {r, g, b} = entry.rgb.from;
                if *pixel_r == r && *pixel_g == g && *pixel_b == b {
                    return (entry.rgb.to, true);
                } else {
                    let Lab { l, a, b} = Lab::from_rgb(&[*pixel_r, *pixel_g, *pixel_b]);
                    let lab_color = LabValue { l, a, b, };
                    let distance = lab_color.delta(entry.lab, DE2000);
                    if best_distance > *distance.value() {
                        best_color = &entry.rgb.to;
                        best_distance = *distance.value();
                    }
                }
            }
            (*best_color, false)
        },
        _ => unreachable!("{:?}", pixel),
    }
}
pub fn replace_rgb_colors(image: &Vec<u8>, colors: &Vec<RgbColorReplacementPair>) -> Result<ConvResult, String> {
    log_1(&format!("wasm data length: {}", image.len()).into());
    log_1(&format!("wasm colors: {:?}", colors).into());
    let color_map: Vec<TranslationCache> = colors.iter().map(|pair| pair.into()).collect();
    let input_image = utils::read(image).unwrap();
    let (width, height) = input_image.dimensions();
    let mut output_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    let mut exact_counter = 0usize;
    let mut nearest_counter = 0usize;
    let mut total_counter = 0usize;
    // let fill: Rgba<u8> = Rgb([0u8, 255u8, 0u8]).to_rgba();
    for (x, y, pixel) in input_image.pixels() {
        total_counter += 1;
        // output_buffer.put_pixel(x, y, rgba2rgb(pixel).to_rgba());
        let rgb_pixel = rgba2rgb(pixel);
        let (color, exact) = lookup(&color_map, &rgb_pixel);
        if exact {
            exact_counter += 1;
        } else {
            nearest_counter += 1;
        }
        output_buffer.put_pixel(x, y, color.to_rgba());
    }
    log_1(&format!("Matching: exact: {exact_counter}, nearest: {nearest_counter}, total: {total_counter}").into());
    let mut output_image: Vec<u8> = Vec::new();
    if let Err(err) = output_buffer.write_to(&mut Cursor::new(&mut output_image), image::ImageOutputFormat::Png) {
        log_1(&err.to_string().into());
        unreachable!("Writing error");
    }
    Ok(ConvResult {
        image: output_image,
        width,
        height,
    })
}