// https://github.com/Bullesta/SpectRust/tree/main
// Importing necessary image processing and screenshot capturing modules.
use image::{DynamicImage, GenericImageView, Pixel, Rgba};
// Function to locate an image on the screen with optional region, minimum confidence, and tolerance.
// Returns coordinates, width, height and confidence if image is found, otherwise None.
fn locate_on_screen(screen: &[Rgba<u8>], img: &[Rgba<u8>], screen_width: u32, screen_height: u32, img_width: u32, img_height: u32, min_confidence: f32, tolerance: u8) -> Option<(u32, u32, u32, u32, f32)> {
    let step_size = 1;

    for y in (0..screen_height - img_height).step_by(step_size) {
        for x in (0..screen_width - img_width).step_by(step_size) {
            let mut matching_pixels = 0;
            let mut total_pixels = 0;

            'outer: for dy in 0..img_height {
                for dx in 0..img_width {
                    let screen_idx: usize = ((y + dy) * screen_width + (x + dx)) as usize;
                    let img_idx: usize = (dy * img_width + dx) as usize;

                    let screen_pixel = screen[screen_idx];
                    let img_pixel = img[img_idx];

                    // Skip transparent pixels
                    if img_pixel[3] < 128 {
                        continue;
                    }

                    total_pixels += 1;

                    if within_tolerance(screen_pixel[0], img_pixel[0], tolerance) &&
                        within_tolerance(screen_pixel[1], img_pixel[1], tolerance) &&
                        within_tolerance(screen_pixel[2], img_pixel[2], tolerance) {
                        matching_pixels += 1;
                    } else {
                        break 'outer;
                    }
                }
            }

            let confidence = if total_pixels == 0 { 0.0 } else { matching_pixels as f32 / total_pixels as f32 };

            if confidence >= min_confidence {
                return Some((x, y, img_width, img_height, confidence));
            }
        }
    }

    None
}

// Helper function to check if a color value is within a tolerance range
fn within_tolerance(value1: u8, value2: u8, tolerance: u8) -> bool {
    let min_value = value2.saturating_sub(tolerance);
    let max_value = value2.saturating_add(tolerance);
    // Check if the color value is within tolerance range
    value1 >= min_value && value1 <= max_value
}


// Function to locate an image on the screen with optional region, minimum confidence, and tolerance.
// Returns coordinates, width, height and confidence if image is found, otherwise None.
pub fn locate_image(screenshot: &DynamicImage, img: &DynamicImage, min_confidence: Option<f32>, tolerance: Option<u8>) -> Option<(u32, u32, u32, u32, f32)> {
    // Default values

    let min_confidence = min_confidence.unwrap_or(0.75);
    let tolerance = tolerance.unwrap_or(25);

    let img_pixels: Vec<_> = img.pixels().map(|p| p.2.to_rgba()).collect();
    let img_width = img.width();
    let img_height = img.height();

    let screen_pixels: Vec<_> = screenshot.pixels().map(|p| p.2.to_rgba()).collect();
    let screen_width = screenshot.width();
    let screen_height = screenshot.height();

    locate_on_screen(
        &screen_pixels,
        &img_pixels,
        screen_width,
        screen_height,
        img_width,
        img_height,
        min_confidence,
        tolerance
    )
}
