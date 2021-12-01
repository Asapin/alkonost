use std::{collections::HashSet, time::Duration};

use alkonost::{Alkonost, AlkonostInMessage, AlkonostOutMessage, DetectorParams, RequestSettings, DecisionAction};
use rillrate::prime::{LiveTail, LiveTailOpts, Pulse, PulseOpts, Table, TableOpts, table::{Row, Col}, Click, ClickOpts};
use tracing::Level;

struct ProcessingStats {
    video_id: String,
    processed_messages: usize,
    suspicios_users_count: usize
}

#[tokio::main]
pub async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let detector_params = DetectorParams::new(4, 5000.0, 5, 30.0, 5, 0.85, 3, 10);
    let request_settings = RequestSettings {
        browser_name: "Firefox".to_string(),
        browser_version: "90.0".to_string(),
        user_agent:
            r#"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:90.0) Gecko/20100101 Firefox/90.0"#
                .to_string(),
    };
    let poll_interval = Duration::from_secs(90);

    let (actor, mut result_rx) =
        match Alkonost::init(detector_params, request_settings, poll_interval) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Error initializing alkonost: {}", &e);
                return;
            }
        };

    match rillrate::install("demo") {
        Ok(_r) => {}
        Err(e) => {
            tracing::error!("Couldn't install RillRate dashboard: {}", &e);
            return;
        }
    };

    let stats_table = Table::new(
        "app.dashboard.stats.Live chats",
        Default::default(),
        TableOpts::default().columns([
            (0, "Chat".into()), 
            (1, "Processed messages".into()),
            (2, "Suspicious users".into())
        ]),
    );

    let active_chat_pulse = Pulse::new(
        "app.dashboard.stats.Chats count",
        Default::default(),
        PulseOpts::default().retain(3600 as u32),
    );
    active_chat_pulse.push(0);

    let decision_log_tail = LiveTail::new(
        "app.dashboard.stats.Decision Log",
        Default::default(),
        LiveTailOpts::default(),
    );

    let click = Click::new(
        "app.dashboard.controls.Close",
        ClickOpts::default().label("Close"),
    );

    let tx_copy = actor.tx.clone();
    click.async_callback(move |envelope| {
        let local_tx = tx_copy.clone();
        async move {
            if envelope.action == None {
                return Ok(());
            }

            tracing::info!("Clicked `Close` button");
            match local_tx.send(AlkonostInMessage::Close).await {
                Ok(_r) => { },
                Err(e) => {
                    tracing::error!("Couldn't send `Close` message: {}", e);
                },
            }
            Ok(())
        }
    });

    let rx_reader = tokio::spawn(async move {
        let mut stats_data = Vec::new();
        while let Some(message) = result_rx.recv().await {
            match message {
                AlkonostOutMessage::NewChat {
                    channel: _,
                    video_id,
                } => {
                    let stats = ProcessingStats {
                        video_id,
                        processed_messages: 0,
                        suspicios_users_count: 0
                    };
                    stats_data.push(stats);
                    stats_table.add_row(Row(stats_data.len() as u64));
                }
                AlkonostOutMessage::ChatClosed {
                    channel: _,
                    video_id,
                } => {
                    let index = stats_data
                        .iter()
                        .position(|stats| &stats.video_id == &video_id);
                    match index {
                        Some(index) => {
                            stats_table.del_row(Row(index as u64));
                            stats_data.remove(index);
                        },
                        None => {
                            tracing::warn!("Chat {} has closed, but it was never opened in the first place", &video_id);
                        },
                    }
                }
                AlkonostOutMessage::DetectorResult {
                    video_id,
                    decisions,
                    processed_messages
                } => {
                    let chat_stats = stats_data
                        .iter_mut()
                        .find(|stats| &stats.video_id == &video_id);
                    match chat_stats {
                        Some(stats) => {
                            stats.processed_messages += processed_messages;

                            for decision in decisions {
                                match decision.action {
                                    DecisionAction::Add(_) => {
                                        stats.suspicios_users_count += 1;
                                    },
                                    DecisionAction::Remove => {
                                        stats.suspicios_users_count -= 1;
                                    }
                                }

                                decision_log_tail.log_now(
                                    &video_id, 
                                    &decision.user,
                                    format!("{:?}", &decision.action)
                                );
                            }
                        },
                        None => {
                            tracing::warn!("Received detector results for {} before `NewChat` message", &video_id);
                        },
                    }
                }
            }
            active_chat_pulse.push(stats_data.len() as f64);
            render_stats_table(&stats_table, &stats_data);
        }

        tracing::info!("rx_reader has been closed");
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
                tracing::error!("Couldn't send message to a stream finder: {}", &e);
                return;
            }
        }
    }

    let _ = actor.join_handle.await;
    let _ = rx_reader.await;

    match rillrate::uninstall() {
        Ok(_r) => {}
        Err(e) => {
            tracing::error!("Couldn't uninstall: {}", &e);
        }
    }

    tracing::info!("Closed");
}

fn render_stats_table(table: &Table, active_chats: &Vec<ProcessingStats>) {
    for (index, stats) in active_chats.iter().enumerate() {
        table.set_cell(Row(index as u64), Col(0), &stats.video_id);
        table.set_cell(Row(index as u64), Col(1), &stats.processed_messages);
        table.set_cell(Row(index as u64), Col(2), &stats.suspicios_users_count);
    }
}