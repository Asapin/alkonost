use std::{collections::HashSet, time::Duration};

use alkonost::{Alkonost, AlkonostInMessage, AlkonostOutMessage, DetectorParams, RequestSettings};
use rillrate::prime::{LiveTail, LiveTailOpts, Pulse, PulseOpts};

#[tokio::main]
pub async fn main() {
    let detector_params = DetectorParams::new(4, 5000.0, 5, 30.0, 5, 0.85, 3, 10);
    let request_settings = RequestSettings {
        browser_name: "Firefox".to_string(),
        browser_version: "90.0".to_string(),
        user_agent:
            r#"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:90.0) Gecko/20100101 Firefox/90.0"#
                .to_string(),
    };
    let poll_interval = Duration::from_secs(90);

    let (actor, mut result_rx) = match Alkonost::init(
        detector_params, 
        request_settings, 
        poll_interval
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("RillRate: Error initializing alkonost: {}", &e);
            return;
        },
    };

    match rillrate::install("demo") {
        Ok(_r) => { },
        Err(e) => {
            println!("RillRate: couldn't install RillRate dashboard: {}", &e);
            return;
        },
    };

    let decision_log_tail = LiveTail::new(
        "app.dashboard.stats.Decision Log",
        Default::default(),
        LiveTailOpts::default(),
    );

    let active_chat_pulse = Pulse::new(
        "app.dashboard.stats.Chats count", 
        Default::default(), 
        PulseOpts::default().retain(3600 as u32)
    );
    active_chat_pulse.push(0);

    let rx_reader = tokio::spawn(async move {
        let mut active_chats = HashSet::new();
        while let Some(message) = result_rx.recv().await {
            match message {
                AlkonostOutMessage::NewChat {
                    channel: _,
                    video_id
                } => {
                    active_chats.insert(video_id);
                },
                AlkonostOutMessage::ChatClosed(video_id) => {
                    active_chats.remove(&video_id);
                },
                AlkonostOutMessage::DetectorResult { 
                    video_id, 
                    decisions 
                } => {
                    for decision in decisions {
                        decision_log_tail.log_now(
                            video_id.clone(), 
                            decision.user, 
                            format!("{:?}", decision.action)
                        );
                    }
                },
            }
            active_chat_pulse.push(active_chats.len() as f64);
        }

        println!("RillRate: rx_reader has been closed");
    });

    let mut channels = HashSet::new();
    channels.insert("UCtMVHI3AJD4Qk4hcbZnI9ZQ"); // SomeOrdinaryGamers
    channels.insert("UC-lHJZR3Gqxm24_Vd_AJ5Yw"); // PewDiePie
    channels.insert("UCqNH56x9g4QYVpzmWTzqVYg"); // Dynamo Gaming
    channels.insert("UCam8T03EOFBsNdR0thrFHdQ"); // VEGETTA777
    channels.insert("UCaHEdZtk6k7SVP-umnzifmQ"); // TheDonato
    channels.insert("UC5c9VlYTSvBSCaoMu_GI6gQ"); // Total Gaming
    channels.insert("UChXi_PlJkRMPYFQBOJ3MpxA"); // Gyan Gaming
    channels.insert("UCSJ4gkVC6NrvII8umztf0Ow"); // Lofi Girl
    channels.insert("UC2wKfjlioOCLP4xQMOWNcgg"); // Typical Gamer
    channels.insert("UCw7FkXsC00lH2v2yB5LQoYA"); // jackfrags
    channels.insert("UCsjTQnlZcSB6fSiP7ht_0OQ"); // Hacks Busters

    for channel_id in channels {
        let message = AlkonostInMessage::AddChannel(channel_id.to_string());
        match actor.tx.send(message).await {
            Ok(_r) => {}
            Err(e) => {
                println!("RillRate: Couldn't send message to a stream finder: {}", &e);
                return;
            }
        }
    }

    let _ = actor.join_handle.await;
    let _ = rx_reader.await;

    match rillrate::uninstall() {
        Ok(_r) => { },
        Err(e) => {
            println!("RillRate: Couldn't uninstall: {}", &e);
        },
    }

    println!("RillRate has been closed");
}