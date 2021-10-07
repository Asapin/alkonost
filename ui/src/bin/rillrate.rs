use std::{collections::HashSet, time::Duration};

use alkonost::{Alkonost, AlkonostMessage, DetectorParams, DetectorResults, RequestSettings, StreamFinderMessages};
use rillrate::prime::{LiveTail, LiveTailOpts, Pulse, PulseOpts};
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
        mut alkonost_rx, 
        handler, 
        stream_finder_tx 
    } = match Alkonost::init(detector_params, request_settings, poll_interval) {
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

    let detector_results = tokio::spawn(async move {
        let mut active_chats = HashSet::new();
        loop {
            match alkonost_rx.recv().await {
                Some(results) => match results {
                    AlkonostMessage::ChatClosed(video_id) => {
                        if active_chats.remove(&video_id) {
                            active_chat_pulse.push(active_chats.len() as f64);
                        }
                    },
                    AlkonostMessage::NewChats(new_chats) => {
                        for chat in new_chats {
                            active_chats.insert(chat);
                        }
                        active_chat_pulse.push(active_chats.len() as f64);
                    },
                    AlkonostMessage::DetectorMessage(detector_result) => {
                        match detector_result {
                            DetectorResults::Close => {
                                return;
                            },
                            DetectorResults::ProcessingResult {
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
                            }
                        }
                    }
                },
                None => {
                    println!("RillRate: Channel from the detector manager has been closed before receiving `Close` message.")
                }
            }
        }
    });

    let mut channels = HashSet::new();
        // 0th Gen
        channels.insert("UCp6993wxpyDPHUpavwDFqgg"); // Tokino Sora
        channels.insert("UCDqI2jOz0weumE8s7paEk6g"); // Robocosan
        channels.insert("UC-hM6YJuNYVAmUWxeIr9FeA"); // Sakura Miko
        channels.insert("UC5CwaMl1eIgY8h02uZw7u8A"); // Hoshimachi Suisei
        channels.insert("UC0TXe_LYZ4scaW2XMyi5_kw"); // AZKi
    
        // 1st Gen
        channels.insert("UCD8HOxPs4Xvsm8H0ZxXGiBw"); // Yozora Mel
        channels.insert("UCdn5BQ06XqgXoAxIhbqw5Rg"); // Shirakami Fubuki
        channels.insert("UCQ0UDLQCjY0rmuxCDE38FGg"); // Natsuiro Matsuri
        channels.insert("UCFTLzh12_nrtzqBPsTCqenA"); // Aki Rosenthal
        channels.insert("UCLbtM3JZfRTg8v2KGag-RMw"); // Aki Rosenthal (sub)
        channels.insert("UC1CfXB_kRs3C-zaeTG3oGyg"); // Akai Haato
        channels.insert("UCHj_mh57PVMXhAUDphUQDFA"); // Akai Haato (sub)
    
        // 2nd Gen
        channels.insert("UC1opHUrw8rvnsadT-iGp7Cg"); // Minato Aqua
        channels.insert("UCXTpFs_3PqI41qX2d9tL2Rw"); // Murasaki Shion
        channels.insert("UC7fk0CB07ly8oSl0aqKkqFg"); // Nakiri Ayame
        channels.insert("UC1suqwovbL1kzsoaZgFZLKg"); // Yuzuki Choco
        channels.insert("UCp3tgHXw_HI0QMk1K8qh3gQ"); // Yuzuki Choco (sub)
        channels.insert("UCvzGlP9oQwU--Y0r9id_jnA"); // Oozora Subaru
    
        // Gamers
        channels.insert("UCp-5t9SrOQwXMU7iIjQfARg"); // Ookami Mio
        channels.insert("UCvaTdHTWBGv3MKj3KVqJVCw"); // Nekomata Okayu
        channels.insert("UChAnqc_AY5_I3Px5dig3X1Q"); // Inugami Korone
    
        // 3rd Gen
        channels.insert("UC1DCedRgGHBdm81E1llLhOQ"); // Usada Pekora
        channels.insert("UCl_gCybOJRIgOXw6Qb4qJzQ"); // Uruha Rushia
        channels.insert("UCvInZx9h3jC2JzsIzoOebWg"); // Shiranui Flare
        channels.insert("UCdyqAaZDKHXg4Ahi7VENThQ"); // Shirogane Noel
        channels.insert("UCCzUftO8KOVkV4wQG1vkUvg"); // Houshou Marine
    
        // 4th Gen
        channels.insert("UCZlDXzGoo7d44bwdNObFacg"); // Amane Kanata
        channels.insert("UCqm3BQLlJfvkTsX_hvm0UmA"); // Tsunomaki Watame
        channels.insert("UC1uv2Oq6kNxgATlCiez59hw"); // Tokoyami Towa
        channels.insert("UCa9Y57gfeY0Zro_noHRVrnw"); // Himemori Luna
        channels.insert("UCS9uQI-jC3DE0L4IpXyvr6w"); // Kiryu Coco
    
        // 5th Gen
        channels.insert("UCFKOVgVbGmX65RxO3EtH3iw"); // Yukihana Lamy
        channels.insert("UCAWSyEs_Io8MtpY3m-zqILA"); // Momosuzu Nene
        channels.insert("UCUKD-uaobj9jiqB-VXt71mA"); // Shishiro Botan
        channels.insert("UCK9V2B22uJYu3N7eR_BT9QA"); // Omaru Polka
        channels.insert("UCgZuwn-O7Szh9cAgHqJ6vjw"); // Mano Aloe
    
        // ID 1st Gen
        channels.insert("UCOyYb1c43VlX9rc_lT6NKQw"); // Ayunda Risu
        channels.insert("UCP0BspO_AMEe3aQqqpo89Dg"); // Hoshinova Moona
        channels.insert("UCAoy6rzhSf4ydcYjJw3WoVg"); // Airani Iofifteen
        
        // ID 2nd Gen
        channels.insert("UCYz_5n-uDuChHtLo7My1HnQ"); // Kureiji Ollie
        channels.insert("UC727SQYUvx5pDDGQpTICNWg"); // Melfissa Anya
        channels.insert("UChgTyjG-pdNvxxhdsXfHQ5Q"); // Pavolia Reine
    
        // EN 1st Gen
        channels.insert("UCL_qhgtOy0dy1Agp8vkySQg"); // Mori Calliope
        channels.insert("UCHsx4Hqa-1ORjQTh9TYDhww"); // Takanashi Kiara
        channels.insert("UCMwGHR0BTZuLsmjY_NT5Pwg"); // Ninomae Ina'nis
        channels.insert("UCoSrY_IQQVpmIRZ9Xf-y93g"); // Gawr Gura
        channels.insert("UCyl1z3jo3XHR1riLFKG5UAg"); // Watson Amelia
        
        // Hope
        channels.insert("UC8rcEBzJSleTkf_-agPM20g"); // IRyS
    
        // EN 2nd Gen
        channels.insert("UCsUj0dszADCGbF3gNrQEuSQ"); // Tsukumo Sana
        channels.insert("UCO_aKKYxn4tvrqPjcTzZ6EQ"); // Ceres Fauna
        channels.insert("UCmbs8T6MWqUHP1tIQvSgKrg"); // Ouro Kronii
        channels.insert("UC3n5uGu18FoCy23ggWWp8tA"); // Nanashi Mumei
        channels.insert("UCgmPnx-EEeOrZSg5Tiw7ZRQ"); // Hakos Baelz
        
        // Official channels
        channels.insert("UCJFZiqLMntJufDCHc6bQixg"); // Hololive
        channels.insert("UCotXwY6s8pWmuWd_snKYjhg"); // Hololive English
        channels.insert("UCfrWoRGlawPQDQxxeIDRP0Q"); // Hololive Indonesia
    
        // Indies
        channels.insert("UCIumx9FItlv6B_JsHVMjVYA"); // Mirea Sheltzs
        channels.insert("UC7YXqPO3eUnxbJ6rN0z2z1Q"); // DELUTAYA Δ
        channels.insert("UC04BxRrX3feybf0W3r1LcDg"); // Hitomi Cocco
        channels.insert("UC22BVlBsZc6ta3Dqz75NU6Q"); // Pochimaru
        // channels.insert("UCdMb33pUEzy-gcoAlaJmVlw"); // Shirono Ten
        channels.insert("UCGhGLjOhmqfVWmq7AiugHyQ"); // Prune
        channels.insert("UC9ruVYPv7yJmV0Rh0NKA-Lw"); // kson ONAIR
        
        
        channels.insert("UCSolk-6yJ_-cJPJLtA4UB0A"); // BlackPantsLegion

    for channel_id in channels {
        let message = StreamFinderMessages::AddChannel(channel_id.to_string());
        match stream_finder_tx.send(message).await {
            Ok(_r) => {}
            Err(e) => {
                println!("RillRate: Couldn't send message to a stream finder: {}", &e);
                return;
            }
        }
    }

    // sleep(Duration::from_secs(130)).await;
    // println!("RillRate: Closing...");
    // match stream_finder_tx.send(StreamFinderMessages::Close).await {
    //     Ok(_r) => { },
    //     Err(e) => {
    //         println!("RillRate: Couldn't send message to a stream finder: {}", &e);
    //         return;
    //     },
    // }
    handler.join().await;
    let _ = detector_results.await;

    match rillrate::uninstall() {
        Ok(_r) => { },
        Err(e) => {
            println!("RillRate: Couldn't uninstall: {}", &e);
        },
    }

    println!("RillRate has been closed");
}