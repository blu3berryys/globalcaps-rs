use futures_util::SinkExt;
use futures_util::StreamExt;
use std::thread;
use std::time::Duration;
use tokio::{self, time};
use tokio_tungstenite::connect_async;
use tungstenite::Utf8Bytes;
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

async fn run_client() {
    let url = "wss://globalcapslock.com/ws".to_string();
    loop {
        match connect_async(url.clone()).await {
            Ok((mut ws_stream, _)) => {
                println!("Connected");
                let mut last_state = capslock::get_capslock_state();
                loop {
                    let current_state = capslock::get_capslock_state();
                    if current_state != last_state {
                        let msg = if current_state { "1" } else { "0" };
                        ws_stream
                            .send(Message::Text(Utf8Bytes::from(msg.to_string())))
                            .await
                            .ok();
                        last_state = current_state;
                    }
                    if let Ok(Some(Ok(Message::Text(data)))) =
                        time::timeout(Duration::from_millis(50), ws_stream.next()).await
                    {
                        match data.as_str() {
                            "1" if !current_state => capslock::set_capslock_state(true),
                            "0" if current_state => capslock::set_capslock_state(false),
                            _ => {}
                        }
                    }
                    time::sleep(Duration::from_millis(50)).await;
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}. Retrying...", e);
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

#[tokio::main]
async fn main() {
    run_client().await;
}
