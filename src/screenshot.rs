use std::io::Cursor;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use async_channel::Sender;
use xcap;
use image;
use image::{DynamicImage, GenericImage, ImageBuffer, ImageFormat, Rgb, RgbImage};
use leptess::LepTess;
use regex::Regex;
use xcap::Monitor;
use crate::UpdateUI;

const MAX_TRIES:i32 = 3;
const SCREENSHOT_DELAY_NS:u32 = 2_000_000_000; // 2 sec
const CREATE_DEBUG_IMAGE:bool = true;

pub fn capture_screen(sender_capture: Arc<Sender<UpdateUI>>) {
    let _ = thread::spawn({
        move || {
            thread::sleep(Duration::new(0, SCREENSHOT_DELAY_NS));
            let binding = xcap::Monitor::all().unwrap();
            let monitor = binding.first().unwrap();
            for i in 0.. MAX_TRIES{
                let start = Instant::now();
                match capture(i,monitor) {
                    None => {println!("capture failed");}
                    Some(delay) => {
                        //TODO display
                        println!("Delay: {:?}",delay);
                        let _ = sender_capture.send_blocking(UpdateUI::DelayMeasured(Some(delay)));
                        break;
                    }
                }
                println!("screenshot to time: {:?}", start.elapsed());
            }
        }
    });
}


fn capture(tries:i32,monitor: &Monitor) -> Option<Duration> {
    let start = Instant::now();
    let image =monitor.capture_image().unwrap();
    println!("Time To Capture: {:?}", start.elapsed());
    let out_file = String::from("debug.jpg");
    let mut image = DynamicImage::ImageRgba8(image).into_rgb8();
    let mut timer_durations = vec![];
    for p in [crate::IMAGE_BYTES_SERVER, crate::IMAGE_BYTES_CLIENT]
    {
        let res = find_timer_spect(p);
        match res {
            Some((x, y, _w, _h, confidence)) => {
                draw_rectangle_on(
                    &mut image,
                    (x - 122, y),
                    (96, 32),
                );
                println!("Image found at {}, {} with confidence {}", x, y, confidence);
                let duration =
                    ocr(image.sub_image(x - 120, y, 94, 32).to_image());

                match duration {
                    Ok(d) => {timer_durations.push(d);}
                    Err(_) => {
                        save_debug_image(&image,out_file,tries);
                        return None;
                    }
                }
            }
            None => { println!("Image not found");

                save_debug_image(&image,out_file,tries);
                return None;
            }
        }
    }
    save_debug_image(&image,out_file,MAX_TRIES);
    if timer_durations[0].as_nanos() == 0 || timer_durations[1].as_nanos() == 0 {
        return None;
    }
    let delay =duration_sub(timer_durations[0],timer_durations[1]);
    Some(delay)
}

fn save_debug_image(image:&RgbImage,path:String, tries:i32){
    if CREATE_DEBUG_IMAGE && tries >= MAX_TRIES {
        image.save(path).unwrap();
    }
}
fn duration_sub(a:Duration, b:Duration) -> Duration{
    let delay;
    if a > b{
        delay = a.saturating_sub(b);
    } else {
        delay = b.saturating_sub(a);
    }
    delay
}


fn ocr(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<Duration,String> {
    let mut lt = LepTess::new(None, "eng").unwrap();

    let mut tiff_buffer = Vec::new();
    image.write_to(
        &mut Cursor::new(&mut tiff_buffer),
        image::ImageOutputFormat::Tiff,
    )
        .unwrap();
    lt.set_image_from_mem(&tiff_buffer).unwrap();

    let res_str = lt.get_utf8_text().unwrap();
    println!("{}", res_str);
    let re =
        Regex::new(r"(?<hour>\d{2}):(?<minutes>\d{2}):(?<seconds>\d{2}).(?<milliseconds>\d{3})")
        .unwrap();
    let Some(caps) = re.captures(res_str.as_str()) else {
        return Err("Could not match Regex Pattern".to_string());
    };

    let millis = caps.name("milliseconds").unwrap().as_str().parse::<u64>().unwrap();
    let seconds = caps.name("seconds").unwrap().as_str().parse::<u64>().unwrap();
    let minutes = caps.name("minutes").unwrap().as_str().parse::<u64>().unwrap();
    let hours = caps.name("hour").unwrap().as_str().parse::<u64>().unwrap();

    const MILLISECOND: u64 = 1000;
    const SECONDS: u64 = 60 * MILLISECOND;
    const MINUTES: u64 = 60 * SECONDS;
    let millis = hours * MINUTES +
        minutes * SECONDS +
        seconds * MILLISECOND +
        millis;

    let time = Duration::from_millis(millis);
    println!("parsed: {:?}", time);
    Ok(time)
}

fn find_timer_spect(pattern: &[u8]) -> Option<(u32, u32, u32, u32, f32)> {
    let img = image::load_from_memory_with_format(pattern, ImageFormat::Jpeg).unwrap();
    let region = None;
    let min_confidence = Some(0.9);
    let tolerance = Some(10);
    let res = spectrust::locate_image(&img, region, min_confidence, tolerance);
    return res;
}

fn draw_rectangle_on(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    (x, y): (u32, u32),
    (w, h): (u32, u32),
) {
    let border_col = Rgb([255u8, 0, 0]);

    const LINE_THICKNESS: u32 = 4;
    // Vertical line at (x,y)
    for off_x in 0..LINE_THICKNESS {
        for off_y in 0..h {
            *img.get_pixel_mut(x + off_x, y + off_y) = border_col;
        }
    }
    // Horizontal line at (x,y)
    for off_y in 0..LINE_THICKNESS {
        for off_x in 0..w {
            *img.get_pixel_mut(x + off_x, y + off_y) = border_col;
        }
    }
    // Vertical line at (x+w,y)
    for off_x in 0..LINE_THICKNESS {
        for off_y in 0..(h + 1) {
            *img.get_pixel_mut(x + off_x + w, y + off_y) = border_col;
        }
    }
    // Horizontal line at (x,y+h)
    for off_y in 0..LINE_THICKNESS {
        for off_x in 0..(w + 1) {
            *img.get_pixel_mut(x + off_x, y + off_y + h) = border_col;
        }
    }
}