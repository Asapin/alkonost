#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use core::{
    http_client::{HttpClient, RequestSettings},
    messages::StreamFinderMessages,
};
use std::{collections::HashSet, sync::Arc, time::Duration};

use chat_manager::chat::ChatManager;
use detector::{detector_params::DetectorParams, DetectorManager};
use stream_finder::StreamFinder;
use tokio::{sync::mpsc, time::sleep};

#[tokio::main]
pub async fn main() {
    let http_client = match HttpClient::init() {
        Ok(init_result) => Arc::new(init_result),
        Err(e) => {
            println!("CLI: Error while creating Http client: {}", &e);
            return;
        }
    };

    let request_settings = RequestSettings {
        browser_name: "Firefox".to_string(),
        browser_version: "90.0".to_string(),
        user_agent:
            r#"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:90.0) Gecko/20100101 Firefox/90.0"#
                .to_string(),
    };

    let (detector_result_tx, mut detector_result_rx) = mpsc::channel(32);
    let params = DetectorParams::new(4, 5000.0, 5, 30.0, 5, 0.85, 3, 10);
    let detector = DetectorManager::init(params, detector_result_tx);

    let detector_results = tokio::spawn(async move {
        loop {
            match detector_result_rx.recv().await {
                Some(results) => match results {
                    core::messages::DetectorResults::Close => {
                        return;
                    }
                    core::messages::DetectorResults::ProcessingResult {
                        video_id,
                        decisions,
                    } => {
                        println!("CLI: <{}>: {:?}", video_id, decisions);
                    }
                    core::messages::DetectorResults::StreamEnded { video_id } => {
                        println!("CLI: stream <{}> ended", video_id);
                    }
                },
                None => {
                    println!("CLI: Channel from the detector manager has been closed before receiving `Close` message.")
                }
            }
        }
    });

    let chat_manager = ChatManager::init(
        http_client.clone(),
        request_settings.clone(),
        detector.tx.clone(),
    );
    let poll_interval = Duration::from_secs(90);
    let stream_finder = StreamFinder::init(
        http_client,
        request_settings,
        chat_manager.tx,
        poll_interval,
    );

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
        match stream_finder.tx.send(message).await {
            Ok(_r) => {}
            Err(e) => {
                println!("CLI: Couldn't send message to a stream finder: {}", &e);
                return;
            }
        }
    }

    // sleep(Duration::from_secs(130)).await;
    // println!("CLI: Closing...");
    // match stream_finder.tx.send(StreamFinderMessages::Close).await {
    //     Ok(_r) => { },
    //     Err(e) => {
    //         println!("CLI: Couldn't send message to a stream finder: {}", &e);
    //         return;
    //     },
    // }

    let _ = detector.join_handle.await;
    let _ = detector_results.await;
    let _ = chat_manager.join_handle.await;
    let _ = stream_finder.join_handle.await;
    println!("CLI closed");
}
