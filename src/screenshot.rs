use std::io::Cursor;
use std::ops::Deref;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use async_channel::Sender;
use xcap;
use image;
use image::{DynamicImage, GenericImage, ImageBuffer, ImageFormat, Rgb, RgbImage};
use leptess::{LepTess, Variable};
use regex::Regex;
use xcap::Monitor;
use crate::UpdateUI;
#[path = "spectrust.rs"] mod spectrust;

const MAX_TRIES:i32 = 3;
const SCREENSHOT_DELAY_NS:u32 = 2_000_000_000; // 2 sec
const CREATE_DEBUG_IMAGE:bool = true;

pub struct CaptureBox {
    width:i32,
    height:i32,
    x_offset:i32,
    y_offset:i32
}

impl CaptureBox {
    pub fn new(width:i32,height:i32,x_offset:i32,y_offset:i32)-> CaptureBox {
        CaptureBox {
            width,
            height,
            x_offset,
            y_offset,
        }
    }
}

pub fn get_monitors() -> Vec<Monitor> {
     xcap::Monitor::all().unwrap()
}

pub fn capture_screen(sender_capture: Arc<Sender<UpdateUI>>, capture_box:Arc<CaptureBox>, monitor_num: usize) {
    let _ = thread::spawn({
        move || {
            thread::sleep(Duration::new(0, SCREENSHOT_DELAY_NS));
            let binding = get_monitors();
            let monitor = &binding.get(monitor_num).unwrap();
            for _i in 0.. MAX_TRIES+1 {
                let start = Instant::now();
                match capture(monitor, capture_box.deref()) {
                    None => {println!("capture failed");}
                    Some(delay) => {
                        println!("Delay: {:?}",delay);
                        let _ = sender_capture.send_blocking(UpdateUI::DelayMeasured(Some(delay)));
                        break;
                    }
                }
                println!("screenshot to time: {:?}", start.elapsed());
            }
            let _ = sender_capture.send_blocking(UpdateUI::DelayMeasured(None));
        }
    });
}


fn capture(monitor: &Monitor, capture_box:&CaptureBox) -> Option<Duration> {
    let start = Instant::now();
    let image = match monitor.capture_image(){
        Ok(x) => x,
        Err(_) =>{
            println!("Error on image capture");
            return None},
    };
    println!("Time To Capture: {:?}", start.elapsed());
    let out_file = String::from("debug.jpg");
    let image = DynamicImage::ImageRgba8(image);
    let mut output_image = image.clone().into_rgb8();
    let mut results = vec![];
    let mut ok = true;
    for p in [crate::IMAGE_BYTES_SERVER, crate::IMAGE_BYTES_CLIENT]
    {
        let res = find_timer_spect(&image,p);
        match res {
            Some((x, y, _w, _h, confidence)) => {
                println!("Image found at {}, {} with confidence {}", x, y, confidence);

                let x = (x as i32 + capture_box.x_offset) as u32;
                let y = (y as i32 + capture_box.y_offset) as u32;
                let duration =
                    ocr(output_image.sub_image(x, y, capture_box.width as u32, capture_box.height as u32).to_image());

                match duration {
                    Ok(d) => {results.push((Some(d),x,y));}
                    Err(e) => {
                        results.push((None,x,y));
                        println!("Error ocr: {:?}",e);
                        ok = false;
                    }
                }
            }
            None => {
                println!("Could not locate program window");
                ok = false;
            }
        }
    }
    save_debug_image(&mut output_image, out_file, MAX_TRIES, &results, &capture_box);
    if !ok {
        return None;
    }

    if results[0].0.unwrap().as_nanos() == 0 || results[1].0.unwrap().as_nanos() == 0 {
        return None;
    }
    let delay = duration_sub(results[0].0.unwrap(),results[1].0.unwrap());
    Some(delay)
}

fn save_debug_image(image: &mut RgbImage, path:String, tries:i32, results:&Vec<(Option<Duration>,u32,u32)>, capture_box:& CaptureBox){
    for i in results{
        draw_rectangle_on(
            image,
            (i.1 , i.2 ),
            (capture_box.width as u32, capture_box.height as u32),
        );
    }

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
        image::ImageFormat::Tiff,
    )
        .unwrap();
    lt.set_variable(Variable::TesseditCharWhitelist,"0123456789.:").expect("error setting tesseract whitelist");
    lt.set_image_from_mem(&tiff_buffer).unwrap();

    let res_str = lt.get_utf8_text().unwrap();
    println!("Ocr: {}", res_str);
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

fn find_timer_spect(screenshot: &DynamicImage, pattern: &[u8]) -> Option<(u32, u32, u32, u32, f32)> {
    let img = image::load_from_memory_with_format(pattern, ImageFormat::Jpeg).unwrap();
    let min_confidence = Some(0.9);
    let tolerance = Some(10);
    let res = spectrust::locate_image(&screenshot,&img, min_confidence, tolerance);
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