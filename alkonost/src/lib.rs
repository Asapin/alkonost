#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use core::http_client::HttpClient;
use std::{sync::Arc, time::Duration};

use chat_manager::chat::ChatManager;
use detector::DetectorManager;
use error::AlkonostInitError;
use stream_finder::StreamFinder;
use tokio::{sync::mpsc::{self, Receiver, Sender}, task::JoinHandle};

pub mod error;
pub type DetectorParams = detector::detector_params::DetectorParams;
pub type RequestSettings = core::http_client::RequestSettings;
pub type DetectorResults = core::messages::DetectorResults;
pub type StreamFinderMessages = core::messages::StreamFinderMessages;

pub struct AlkonostHandle {
    detector_handler: JoinHandle<()>,
    chat_manager_handler: JoinHandle<()>,
    stream_finder_handler: JoinHandle<()>,
}

impl AlkonostHandle {
    pub async fn join(self) {
        let _ = self.detector_handler.await;
        let _ = self.chat_manager_handler.await;
        let _ = self.stream_finder_handler.await;
    }
}

pub struct Alkonost {
    pub detector_rx: Receiver<DetectorResults>,
    pub handler: AlkonostHandle,
    pub stream_finder_tx: Sender<StreamFinderMessages>
}

impl Alkonost {
    pub fn init(
        detector_params: DetectorParams,
        request_settings: RequestSettings,
        chat_poll_interval: Duration
    ) -> Result<Self, AlkonostInitError> {
        let http_client = Arc::new(HttpClient::init()?);
        let (detector_result_tx, detector_result_rx) = mpsc::channel(32);
        let detector = DetectorManager::init(detector_params, detector_result_tx);
        let chat_manager = ChatManager::init(
            http_client.clone(), 
            request_settings.clone(), 
            detector.tx
        );
        let stream_finder = StreamFinder::init(
            http_client, 
            request_settings, 
            chat_manager.tx, 
            chat_poll_interval
        );

        let alkonost_handler = AlkonostHandle {
            detector_handler: detector.join_handle,
            chat_manager_handler: chat_manager.join_handle,
            stream_finder_handler: stream_finder.join_handle,
        };

        Ok(Self {
            detector_rx: detector_result_rx,
            handler: alkonost_handler,
            stream_finder_tx: stream_finder.tx
        })
    }
}