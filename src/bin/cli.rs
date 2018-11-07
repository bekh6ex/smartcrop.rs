extern crate smartcrop;

use smartcrop::{CropSettings, Analyzer};

extern crate clap;

use clap::{Arg, App};
use std::num::NonZeroU32;

extern crate image;

fn main() {
    let an: Analyzer = Analyzer::new(CropSettings::default());

    let matches = App::new("asd")
        .arg(Arg::with_name("INPUT").required(true))
        .arg(Arg::with_name("OUTPUT").required(true))
        .get_matches();

    let file_in = matches.value_of("INPUT").unwrap();
    let file_out = matches.value_of("OUTPUT").unwrap();

    // Use the open function to load an image from a Path.
    // ```open``` returns a `DynamicImage` on success.
    let mut img = image::open(file_in).unwrap();

    let crop = an.find_best_crop(&img, NonZeroU32::new(10).unwrap(), NonZeroU32::new(10).unwrap()).unwrap().crop;

    let cropped = img.crop(crop.x, crop.y, crop.width, crop.height);

    cropped.save(file_out).unwrap();

    println!("{:?}", crop)
}