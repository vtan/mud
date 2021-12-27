use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use log::debug;
use serde::Deserialize;
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};

use crate::{id::Id, server_actor};

static NEXT_PLAYER_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize, Debug)]
pub struct ConnectQuery {
    name: String,
}

pub async fn handle_connection(
    ws: WebSocket,
    connect_query: ConnectQuery,
    actor_sender: mpsc::Sender<server_actor::Message>,
) {
    debug!("New connection");
    let player_id = Id::new(NEXT_PLAYER_ID.fetch_add(1, Ordering::SeqCst));
    let (mut sink, mut stream) = ws.split();

    let (event_sender, mut event_receiver) = mpsc::channel::<server_actor::PlayerEvent>(64);
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            sink.send(Message::text(json)).await.unwrap();
        }
        sink.close().await.unwrap();
        debug!("Sender closed");
    });

    actor_sender
        .send(server_actor::Message::PlayerConnected {
            player_id,
            player_name: connect_query.name,
            connection: event_sender,
        })
        .await
        .unwrap();

    while let Some(Ok(message)) = stream.next().await {
        if let Ok(text) = message.to_str() {
            actor_sender
                .send(server_actor::Message::PlayerCommand { player_id, command: text.to_string() })
                .await
                .unwrap();
        } else {
            break;
        }
    }
    actor_sender
        .send(server_actor::Message::PlayerDisconnected { player_id })
        .await
        .unwrap();
    debug!("Receiver closed");
}
