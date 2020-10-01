use channels_lite::channels::{channel_author, channel_subscriber, Network};
use channels_lite::utils::payload::json::PayloadBuilder;
use failure::Fallible;
use serde::{Deserialize, Serialize};
use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

///
/// Some example of sensor Data
///
#[derive(Serialize, Debug, Deserialize)]
pub struct SensorData {
    ts: u64,
    presure: f32,
}

impl SensorData {
    pub fn new(presure: f32) -> Self {
        SensorData {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            presure: presure,
        }
    }
}

#[tokio::main]
async fn main() -> Fallible<()> {
    let seed_author = None;
    let seed_subscriber = None;
    let delay_time: u64 = 40;

    //Create Channel Instance for author
    let mut channel_author = channel_author::Channel::new(Network::Main, seed_author);

    //Open Channel
    let (channel_address, announcement_tag) = channel_author.open().unwrap();
    println!("Author: Announced channel: {} ", channel_address);

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    //Create Channel Instance for subscriber
    let mut channel_subscriber = channel_subscriber::Channel::new(
        Network::Devnet,
        channel_address,
        announcement_tag,
        seed_subscriber,
    );

    //Connect to channel
    let subscription_tag = channel_subscriber.connect().unwrap();
    println!("Subscriber: Connected to channel: {}", subscription_tag);

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    //Add subscriber
    let keyload_tag = channel_author.add_subscriber(subscription_tag).unwrap();
    println!("Author: keyload_tag ID: {} ", keyload_tag);

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    channel_subscriber.update_keyload(keyload_tag).unwrap();
    println!("Subscriber: Updated keyload");

    //Write signed public message
    let s0 = channel_author
        .write_signed(PayloadBuilder::new().public(&SensorData::new(1.0))?.build())
        .unwrap();
    println!("Author: Sent signed public message: {}", s0);

    //Write signed masked message
    let s1 = channel_author
        .write_signed(
            PayloadBuilder::new()
                .masked(&SensorData::new(19.0))?
                .build(),
        )
        .unwrap();
    println!("Author: Sent signed masked message: {}", s1);

    //Write tagged message
    let s2 = channel_author
        .write_tagged(
            PayloadBuilder::new()
                .public(&SensorData::new(17.0))?
                .masked(&SensorData::new(19.0))?
                .build(),
        )
        .unwrap();
    println!("Author: Sent tagged message: {}", s2);

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time * 2);
    thread::sleep(Duration::from_secs(delay_time * 2));

    let tags = channel_subscriber.get_next_message();

    //Read all signed messages
    let list_signed_public: Vec<(Option<String>, Option<String>)> = channel_subscriber
        .read_signed(tags[1].clone().unwrap())
        .unwrap();
    println!("Subscriber: Reading signed public messages");
    for msg in list_signed_public.iter() {
        let (public, masked) = msg;
        println!(
            "Subscriber: Found Signed Public Message -> Public: {:?} -- Masked: {:?}",
            public, masked
        )
    }

    let list_signed_masked: Vec<(Option<String>, Option<String>)> = channel_subscriber
        .read_signed(tags[2].clone().unwrap())
        .unwrap();
    println!("Subscriber: Reading signed masked messages");
    for msg in list_signed_masked.iter() {
        let (public, masked) = msg;
        println!(
            "Subscriber: Found Signed Masked Message -> Public: {:?} -- Masked: {:?}",
            public, masked
        )
    }

    //Read all tagged messages
    let list_tagged: Vec<(Option<String>, Option<String>)> = channel_subscriber
        .read_tagged(tags[3].clone().unwrap())
        .unwrap();
    println!("Subscriber: Reading tagged messages");
    for msg in list_tagged.iter() {
        let (public, masked) = msg;
        println!(
            "Subscriber: Found Tagged Message -> Public: {:?} -- Masked: {:?}",
            public, masked
        )
    }

    /*
    //Disconnect from channel
    let unsubscribe_tag = channel_subscriber.disconnect().unwrap();
    println!("Subscriber: Disconnected from channel");

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    channel_author.remove_subscriber(unsubscribe_tag).unwrap();
    println!("Author: Removed subscriber");
    */

    Ok(())
}
