extern crate image as image_ext;

use self::image_ext::{imageops, FilterType, GenericImage, ImageBuffer, Pixel};
use super::Image;
use super::ResizableImage;
use super::RGB;
use std;

impl<I, P> Image for I
where
    I: GenericImage<Pixel = P> + 'static,
    P: Pixel<Subpixel = u8> + 'static,
{
    fn width(&self) -> u32 {
        GenericImage::width(self)
    }

    fn height(&self) -> u32 {
        GenericImage::height(self)
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        let px = self.get_pixel(x, y).to_rgb();

        let r = px[0];
        let g = px[1];
        let b = px[2];
        RGB { r, g, b }
    }
}

impl<I, P> ResizableImage<ImageBuffer<P, std::vec::Vec<u8>>> for I
where
    I: GenericImage<Pixel = P> + 'static,
    P: Pixel<Subpixel = u8> + 'static,
{
    fn resize(&self, width: u32, height: u32) -> ImageBuffer<P, std::vec::Vec<u8>> {
        imageops::resize(self, width, height, FilterType::Lanczos3)
    }
}
