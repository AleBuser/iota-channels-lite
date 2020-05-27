//use channels_lite::channels::ResponseWriteSigned;
use channels_lite::channels::{channel_author, channel_subscriber};
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
    let seed_author = "SOME9AUTHOR9SEED9SECRTE9UKOL";
    let seed_subscriber = "SOME9SUBSCRIBER9SEETKEW";

    let node: &'static str = "https://nodes.devnet.iota.org:443";

    let delay_time: u64 = 40;

    //Create Channel Instance for author
    let mut channel_author = channel_author::Channel::new(seed_author, node);

    //Open Channel
    let (channel_address, announcement_tag) = channel_author.open().unwrap();
    println!("Author: Announced channel");

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    //Create Channel Instance for subscriber
    let mut channel_subscriber =
        channel_subscriber::Channel::new(seed_subscriber, node, channel_address, announcement_tag);

    //Connect to channel
    let subscription_tag = channel_subscriber.connect().unwrap();
    println!("Subscriber: Connected to channel");

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    //Add subscriber
    let keyload_tag = channel_author.add_subscriber(subscription_tag).unwrap();
    println!("Author: keyload_tag");

    //Write signed public message
    let response_write_signed = channel_author
        .write_signed(
            false,
            PayloadBuilder::new().public(&SensorData::new(1.0))?.build(),
        )
        .unwrap();

    let signed_packed_tag_public = response_write_signed.signed_message_tag;
    let change_key_tag_public = response_write_signed.change_key_tag;
    println!("Author: Sent signed public message");

    //Write signed masked message
    let response_write_signed = channel_author
        .write_signed(
            false,
            PayloadBuilder::new()
                .masked(&SensorData::new(19.0))?
                .build(),
        )
        .unwrap();
    let signed_packed_tag_masked = response_write_signed.signed_message_tag;
    let change_key_tag_masked = response_write_signed.change_key_tag;
    println!("Author: Sent signed masked message");

    //Write tagged message
    let tagged_packed_tag: String = channel_author
        .write_tagged(
            PayloadBuilder::new()
                .public(&SensorData::new(17.0))?
                .masked(&SensorData::new(19.0))?
                .build(),
        )
        .unwrap();
    println!("Author: Sent tagged message");

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time * 2);
    thread::sleep(Duration::from_secs(delay_time * 2));

    channel_subscriber.update_keyload(keyload_tag).unwrap();
    println!("Subscriber: Updated keyload");

    //Read all signed messages
    let list_signed_public: Vec<(Option<SensorData>, Option<SensorData>)> = channel_subscriber
        .read_signed(signed_packed_tag_public, change_key_tag_public)
        .unwrap();
    println!("Subscriber: Reading signed public messages");
    for msg in list_signed_public.iter() {
        let (public, masked) = msg;
        println!(
            "Subscriber: Found Signed Public Message -> Public: {:?} -- Masked: {:?}",
            public, masked
        )
    }

    let list_signed_masked: Vec<(Option<SensorData>, Option<SensorData>)> = channel_subscriber
        .read_signed(signed_packed_tag_masked, change_key_tag_masked)
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
    let list_tagged: Vec<(Option<SensorData>, Option<SensorData>)> =
        channel_subscriber.read_tagged(tagged_packed_tag).unwrap();
    println!("Subscriber: Reading tagged messages");
    for msg in list_tagged.iter() {
        let (public, masked) = msg;
        println!(
            "Subscriber: Found Tagged Message -> Public: {:?} -- Masked: {:?}",
            public, masked
        )
    }

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    // channel_subscriber.update_keyload(change_key_tag).unwrap();
    // println!("Subscriber: Updated key for channel");

    //Disconnect from channel
    let unsubscribe_tag = channel_subscriber.disconnect().unwrap();
    println!("Subscriber: Disconnected from channel");

    //Give messages some time to propagate
    println!("Waiting for propagation... ({}s)", delay_time);
    thread::sleep(Duration::from_secs(delay_time));

    channel_author.remove_subscriber(unsubscribe_tag).unwrap();
    println!("Author: Removed subscriber");

    Ok(())
}
