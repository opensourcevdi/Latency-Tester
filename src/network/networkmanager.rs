use std::ops::Deref;
use std::thread;
use message_io::network::{NetEvent, Transport, RemoteAddr, Endpoint};
use message_io::node::{self, NodeHandler};
use std::sync::{Arc, Mutex};
use async_channel::Sender;
use super::messages::NetworkMessage;
use crate::UpdateUI;

#[derive(Copy, Clone)]
enum ConnectionState {
    Idle,
    Connecting,
    Connected,
}

enum Mode {
    Server(Option<Endpoint>),
    Client(Option<Endpoint>),
    Unknown,
}

pub struct NetworkManager {
    mode: Arc<Mutex<Mode>>,
    handler: Mutex<Option<NodeHandler<()>>>,
}

impl NetworkManager {
    pub fn send(&self, message: NetworkMessage) {
        let output_data = bincode::serialize(&message).unwrap();
        let mut_mode = self.mode.lock().unwrap();
        let endpoint = {
            match *mut_mode {
                Mode::Server(h) => h.expect("No Endpoint"),
                Mode::Client(h) => h.expect("No Endpoint"),
                _ => return,
            }
        };
        let mut_handler = self.handler.lock().unwrap();
        mut_handler.deref().as_ref().unwrap()
            .network().send(endpoint, &output_data);
    }
    pub fn new() -> NetworkManager {
        NetworkManager {
            mode: Arc::new(Mutex::from(Mode::Unknown)),
            handler: Mutex::from(None),
        }
    }

    pub fn connect(&self, is_client: bool, transport: Transport, remote_addr: RemoteAddr,
                   sender_ui_channel: Arc<Sender<UpdateUI>>) -> () {
        let (handler, listener) = node::split::<()>();
        {
            let mut mut_handler = self.handler.lock().unwrap();
            *mut_handler = Some(handler.clone());
        }
        if is_client {
            let (server_id, _) =
                handler.network().connect(transport, remote_addr.clone()).unwrap();
            {
                let mut mut_mode = self.mode.lock().unwrap();
                *mut_mode = Mode::Client(Some(server_id))
            }
        } else {
            match handler.network().listen(transport, remote_addr.clone()) {
                Ok((_id, real_addr)) => println!("Server running at {} by {}",
                                                 real_addr, transport),
                Err(_) => {
                    println!("Can not listening at {} by {}", remote_addr, transport);
                    return;
                }
            }
            {
                let mut mut_mode = self.mode.lock().unwrap();
                *mut_mode = Mode::Server(None)
            }
        }
        let mode = self.mode.clone();
        let _ = thread::spawn({
            let handler = handler.clone();
            move || {
                listener.for_each(move |event| match event.network() {
                    NetEvent::Connected(e, _) => {
                        if matches!(mode.deref().lock().unwrap().deref(), Mode::Client(_)) {
                            {
                                let output_data = bincode::serialize(&NetworkMessage::Ping)
                                    .unwrap();
                                handler.network().send(e,&output_data);
                            }
                        }
                    }
                    NetEvent::Accepted(_, _) => {}
                    NetEvent::Message(e, input_data) => {
                        let message: NetworkMessage = bincode::deserialize(&input_data).unwrap();
                        match message {
                            NetworkMessage::StartTimer => {
                                sender_ui_channel.deref().send_blocking(UpdateUI::StartTimer(
                                    std::time::Instant::now())).unwrap()
                            }
                            NetworkMessage::Ping => {
                                let output_data = bincode::serialize(&NetworkMessage::Pong)
                                    .unwrap();
                                handler.network().send(e, &output_data);
                                if matches!(mode.deref().lock().unwrap().deref(), Mode::Server(_)) {
                                    {
                                        let mut mut_mode = mode.lock().unwrap();
                                        *mut_mode = Mode::Server(Some(e));
                                    }
                                }
                            }
                            NetworkMessage::Pong => {}
                            NetworkMessage::ResetTimer=>{
                                sender_ui_channel.deref().send_blocking(UpdateUI::ResetTimer).unwrap()
                            }
                            _ => {}
                        }
                    }
                    NetEvent::Disconnected(_) => {
                        println!("Server is disconnected");
                        handler.stop();
                    }
                });
            }
        });
    }
}