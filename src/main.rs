mod screenshot;
mod network {
    pub mod networkmanager;
    pub mod messages;
}

use std::ops::Deref;
use std::thread::sleep;
use std::time::{Duration, Instant};
use gtk4 as gtk;
use gtk::{glib, Label, ListBox, prelude::*};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use async_channel::Sender;
use gtk4::gdk_pixbuf::{PixbufLoader};
use gtk4::{Align, gio, Image, ListStore, PolicyType, ScrolledWindow};
use message_io::network::{ToRemoteAddr, Transport};
use crate::network::messages::NetworkMessage;
use crate::screenshot::capture_screen;
use chrono::Local;

enum UpdateUI {

    SetTimer(String),
    StartTimer(Instant),
    DelayMeasured(Option<Duration>),
    ResetTimer,
    Ping(Duration)
}

struct Measurement{

}

pub static IMAGE_BYTES_SERVER: &'static [u8] = include_bytes!("resources/server.jpg");
pub static IMAGE_BYTES_CLIENT: &'static [u8] = include_bytes!("resources/desktop.jpg");

fn main() -> glib::ExitCode {
    let application = gtk::Application::builder()
        .application_id("de.rz.latency_test")
        .build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title(Some("Latency Tester"));
    window.set_default_size(600, 300);
    let grid = gtk::Grid::builder()
        .margin_start(20)
        .margin_end(20)
        .margin_top(6)
        .margin_bottom(6)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .row_spacing(10)
        .column_spacing(10)


        .build();
    window.set_child(Some(&grid));

    let time = elapsed_to_string(&Instant::now());
    let label = Label::default();
    label.set_text(&time);

    let start_button = gtk::Button::builder()
        .label("Start")
        .build();
    let addr = gtk::Entry::builder()
        .text("127.0.0.1:4999")
        .build();
    let button_connect = gtk::Button::builder()
        .label("Connect")
        .build();
    let button_listen = gtk::Button::builder()
        .label("Listen")
        .build();
    let status_image = Arc::new( Image::builder()
        .width_request(32)
        .height_request(32)
        .pixel_size(1)
        .build());
    let label_text_ping = Label::builder()
        .valign(Align::Start)
        .build();
    label_text_ping.set_text("Ping: ");

    let label_ping = Label::builder()
        .valign(Align::Start)
        .build();
    label_ping.set_text("-");

    let run_stopwatch = Arc::new(AtomicBool::new(false));

    let (sender, receiver)
        = async_channel::bounded::<UpdateUI>(10);

    let sender = Arc::new(sender);
    let network
        = Arc::new(network::networkmanager::NetworkManager::new());

    let network_connect = network.clone();
    let network_client_connect = network.clone();
    let network_ui_update = network.clone();
    let sender_start = sender.clone();
    let sender_capture = sender.clone();
    let run_stopwatch_start = run_stopwatch.clone();
    let start_t = move |inst: Instant| {
        let run_stopwatch = Arc::clone(&run_stopwatch_start);
        let sender_start = Arc::clone(&sender_start);
        start_timer(run_stopwatch, sender_start, inst);
    };
    let start_2 = start_t.clone();

    start_button.connect_clicked(move |_| {
        network.send(NetworkMessage::StartTimer);
        start_2(Instant::now());
        let sender_capture = Arc::clone(&sender_capture);
        capture_screen(sender_capture);


    });

    let sender_connect = sender.clone();
    let addr2 = addr.clone();

    let status_image_clone = Arc::clone(&status_image);
    button_connect.connect_clicked(move |_| {

        let sender = Arc::clone(&sender_connect);
        let network_client_connect = Arc::clone(&network_client_connect);
        network_client_connect.connect(true, Transport::Udp,
                                addr2.text().as_str().to_remote_addr().unwrap(), sender);
        set_image(status_image_clone.deref(), IMAGE_BYTES_CLIENT);
    });

    let addr3 = addr.clone();
    let status_image_3 = Arc::clone(&status_image);

    button_listen.connect_clicked(move |_| {

        let sender = Arc::clone(&sender);
        let network_connect = Arc::clone(&network_connect);
        network_connect.connect(false, Transport::Udp,
                               addr3.text().as_str().to_remote_addr().unwrap(), sender);
        set_image(status_image_3.deref(), IMAGE_BYTES_SERVER);
    });


    let (scrolled_window, list_box) =add_delay_listbox();

    grid.attach(&label, 0, 0, 1, 1);
    grid.attach(status_image.deref(), 1, 0, 1, 1);
    grid.attach(&start_button, 1, 1, 1, 1);
    grid.attach(&addr, 0, 2, 2, 1);
    grid.attach(&button_connect, 0, 3, 1, 1);
    grid.attach(&button_listen, 1, 3, 1, 1);
    grid.attach(&scrolled_window,4,0,3,4);
    grid.attach(&label_text_ping,0,5,1,1);
    grid.attach(&label_ping,1,5,1,1);
    // Spawn a future on main context and set the text buffer text from here
    glib::MainContext::default().spawn_local(async move {
        while let Ok(message) = receiver.recv().await {
            match message {
                UpdateUI::SetTimer(text) => { label.set_text(text.as_str()); }
                UpdateUI::StartTimer(inst) => { start_t(inst); }
                UpdateUI::ResetTimer=> {
                    run_stopwatch.store(false,Ordering::Relaxed);
                }
                UpdateUI::DelayMeasured(x) => {
                    run_stopwatch.store(false,Ordering::Relaxed);
                    network_ui_update.send(NetworkMessage::ResetTimer);
                    match x {

                        None => {

                        }
                        Some(d) => {
                            let label = Label::new(Some(format!("{}: {:?}",Local::now().format("%X"),d).as_str()));

                            list_box.append(&label);
                        }
                    }

                }
                UpdateUI::Ping(p) => {
                    label_ping.set_text(format!("{:?}",p).as_str());
                }
            }
        }
    });
    window.present();
}

fn set_image(image: &Image, image_data: &[u8]){
    let loader = PixbufLoader::with_type("jpeg").unwrap();
    loader.write(image_data).unwrap();
    loader.close().unwrap();
    let pixbuf = loader.pixbuf().unwrap();
    image.set_from_pixbuf(Some(&pixbuf));
}
fn start_timer(run_stopwatch: Arc<AtomicBool>, sender: Arc<Sender<UpdateUI>>, inst: Instant) {
    run_stopwatch.store(true,Ordering::Relaxed);

    thread::spawn(move || {
        timer_update(sender, run_stopwatch, inst);
    });

}


fn add_delay_listbox() -> (ScrolledWindow, ListBox) {




    let listbox = ListBox::builder()
        .build();

    listbox.set_width_request(200);

    let scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(200)
        .child(&listbox)
        .build();
    scrolled_window.set_child(Some(&listbox));
    (scrolled_window, listbox)
}





fn timer_update(sender: Arc<Sender<UpdateUI>>, run: Arc<AtomicBool>, inst: Instant) {
    loop {
        if !run.load(Ordering::Relaxed) {
            let _ = sender.deref().send_blocking(UpdateUI::SetTimer(elapsed_to_string(&Instant::now())))
                .expect("timer channel closed");
            break;
        }
        let _ = sender.deref().send_blocking(UpdateUI::SetTimer(elapsed_to_string(&inst)))
            .expect("timer channel closed");
        sleep(Duration::from_millis(4));
    }

}

fn elapsed_to_string(instant: &Instant) -> String {
    let seconds = instant.elapsed().as_secs();
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    format!(
        "{:0>2}:{:0>2}:{:0>2}.{:0>3}",
        seconds / HOUR,
        (seconds % HOUR) / MINUTE,
        seconds % MINUTE,
        instant.elapsed().subsec_millis(),
    )
}