use std::{collections::HashSet, time::Duration};

use alkonost::{Alkonost, DetectorParams, DetectorResults, RequestSettings, StreamFinderMessages};
use tokio::time::sleep;

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

    let Alkonost {
        mut detector_rx, 
        handler, 
        stream_finder_tx 
    } = match Alkonost::init(detector_params, request_settings, poll_interval) {
        Ok(r) => r,
        Err(e) => {
            println!("CLI: Error initializing alkonost: {}", &e);
            return;
        },
    };

    let detector_results = tokio::spawn(async move {
        loop {
            match detector_rx.recv().await {
                Some(results) => match results {
                    DetectorResults::Close => {
                        return;
                    }
                    DetectorResults::ProcessingResult {
                        video_id,
                        decisions,
                    } => {
                        println!("CLI: <{}>: {:?}", video_id, decisions);
                    }
                    DetectorResults::StreamEnded { video_id } => {
                        println!("CLI: stream <{}> ended", video_id);
                    }
                },
                None => {
                    println!("CLI: Channel from the detector manager has been closed before receiving `Close` message.")
                }
            }
        }
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
        let message = StreamFinderMessages::AddChannel(channel_id.to_string());
        match stream_finder_tx.send(message).await {
            Ok(_r) => {}
            Err(e) => {
                println!("CLI: Couldn't send message to a stream finder: {}", &e);
                return;
            }
        }
    }

    sleep(Duration::from_secs(130)).await;
    println!("CLI: Closing...");
    match stream_finder_tx.send(StreamFinderMessages::Close).await {
        Ok(_r) => { },
        Err(e) => {
            println!("CLI: Couldn't send message to a stream finder: {}", &e);
            return;
        },
    }
    handler.join().await;
    let _ = detector_results.await;

    println!("CLI has been closed");
}