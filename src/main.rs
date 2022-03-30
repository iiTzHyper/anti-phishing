use futures_util::stream::StreamExt;
use serde::{
    Serialize, 
    Deserialize
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    sync::Arc
};
use twilight_gateway::{
    cluster::ClusterBuilder,
    Event,
    Intents
};
use twilight_http::Client;

#[derive(Serialize, Deserialize, Debug)]
struct Match {
    followed: bool,
    domain: String,
    url: String,
    source: String,
    r#type: String,
    trust_rating: f32
}

#[derive(Serialize, Deserialize, Debug)]
struct PhishingAPIResponse {
    r#match:  bool,
    matches: Option<Vec<Match>>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Initializing Bot");

    let token = env::var("TOKEN")?;

    let http_client = Arc::new(Client::new(token.clone()));

    let (cluster, mut events) = ClusterBuilder::new(
        token,
        Intents::GUILDS
            | Intents::GUILD_MESSAGES
            | Intents::MESSAGE_CONTENT,
    )
        .http_client(http_client.clone())
        .build()
        .await?;

    cluster.up().await;

    while let Some(event) = events.next().await {
        let http_client = http_client.clone();

        tokio::spawn(async move {
            if handle_event(http_client.clone(), event.clone())
                .await
                .is_err()
            {
                    match event.1 {
                        Event::MessageCreate(message) => {
                            let _message = http_client
                                .create_message(message.channel_id)
                                .content("Oops, there's an error")
                                .unwrap()
                                .exec()
                                .await;
                        }
                        _ => println!("Error from event we don't handle")
                    }
            }
        });
    }

    Ok(())
}

async fn handle_event(
    http_client: Arc<Client>,
    (shard_id, event): (u64, Event),
) -> Result<(), Box<dyn Error>> {
    match event {
        Event::Ready(_) => {
            println!("Shard {} Connected", shard_id);
        }
        Event::MessageCreate(msg) => {
            let mut map = HashMap::new();
            map.insert("message", &msg.content);

            println!("received message: {}", msg.content);

            let client = reqwest::Client::new();
            let http_res = client.post("https://anti-fish.bitflow.dev/check")
                .json(&map)
                .header(reqwest::header::USER_AGENT, "Rust Bot")
                .send()
                .await?;

            let res = http_res.json::<PhishingAPIResponse>().await;
            if res.unwrap().r#match == true {
                http_client.delete_message(msg.channel_id, msg.id)
                    .exec()
                    .await?;
            }
        }
        _ => {}
    }

    Ok(())
}