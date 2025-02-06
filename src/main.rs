use futures_util::SinkExt;
use futures_util::StreamExt;
use std::thread;
use std::time::Duration;
use tokio::{self, time};
use tokio_tungstenite::connect_async;
use tracing::{Level, debug, error, info, instrument};
use tungstenite::protocol::Message;

#[cfg(target_os = "windows")]
mod capslock {
    use winapi::um::winuser::{
        GetKeyState, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_CAPITAL, keybd_event,
    };
    pub fn get_capslock_state() -> bool {
        unsafe { GetKeyState(VK_CAPITAL) & 1 != 0 }
    }
    pub fn set_capslock_state(enabled: bool) {
        if get_capslock_state() != enabled {
            unsafe {
                keybd_event(VK_CAPITAL as u8, 0x45, KEYEVENTF_EXTENDEDKEY, 0);
                keybd_event(
                    VK_CAPITAL as u8,
                    0x45,
                    KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP,
                    0,
                );
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod capslock {
    use std::process::Command;
    pub fn get_capslock_state() -> bool {
        let output = Command::new("xset").arg("-q").output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("Caps Lock:   on")
    }
    pub fn set_capslock_state(enabled: bool) {
        let state = get_capslock_state();
        if state != enabled {
            Command::new("xdotool")
                .arg("key")
                .arg("Caps_Lock")
                .output()
                .unwrap();
        }
    }
}

#[instrument]
async fn run_client() {
    let url = "wss://globalcapslock.com/ws".to_string();
    loop {
        match connect_async(url.clone()).await {
            Ok((mut ws_stream, _)) => {
                info!("Connected to server");
                let mut last_state = capslock::get_capslock_state();
                info!("Initial Caps Lock state: {}", last_state);

                loop {
                    let current_state = capslock::get_capslock_state();

                    // Send state update to server
                    if current_state != last_state {
                        let msg = if current_state { "1" } else { "0" };
                        info!("Caps Lock state changed to: {}", msg);

                        match ws_stream.send(Message::Text(msg.to_string().into())).await {
                            Ok(_) => debug!("Successfully sent state update"),
                            Err(e) => {
                                error!("Failed to send state update: {}", e);
                                break;
                            }
                        }
                        last_state = current_state;
                    }

                    // Receive server updates
                    match time::timeout(Duration::from_millis(50), ws_stream.next()).await {
                        Ok(Some(Ok(Message::Text(data)))) => {
                            debug!("Received server message: {}", data);
                            match data.as_str() {
                                "1" if !current_state => {
                                    info!("Setting Caps Lock ON per server request");
                                    capslock::set_capslock_state(true);
                                }
                                "0" if current_state => {
                                    info!("Setting Caps Lock OFF per server request");
                                    capslock::set_capslock_state(false);
                                }
                                _ => debug!("Ignoring server message: {}", data),
                            }
                        }
                        Ok(Some(Err(e))) => {
                            error!("WebSocket receive error: {}", e);
                            break;
                        }
                        Err(_) => debug!("No message received in timeout period"),
                        _ => {}
                    }

                    time::sleep(Duration::from_millis(50)).await;
                }
            }
            Err(e) => {
                error!("Connection error: {}. Retrying...", e);
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting GlobalCaps :3");

    run_client().await;
}
