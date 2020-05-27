#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
use crate::utils::payload::json::Payload;
use failure::Fallible;
use iota_lib_rs::prelude::iota_client;
use iota_streams::app::transport::tangle::client::SendTrytesOptions;
use iota_streams::app::transport::Transport;
use iota_streams::app_channels::{
    api::tangle::{Address, Subscriber},
    message,
};
use serde::de::DeserializeOwned;

pub struct Channel {
    subscriber: Subscriber,
    is_connected: bool,
    send_opt: SendTrytesOptions,
    client: iota_client::Client<'static>,
    announcement_link: Address,
    subscription_link: Address,
    channel_address: String,
}

impl Channel {
    pub fn new(
        seed: &str,
        node_ulr: &'static str,
        channel_address: String,
        announcement_tag: String,
    ) -> Channel {
        let subscriber = Subscriber::new(seed, true);

        let mut options = SendTrytesOptions::default();
        options.min_weight_magnitude = 9;
        options.local_pow = false;

        Self {
            subscriber: subscriber,
            is_connected: false,
            send_opt: options,
            client: iota_client::Client::new(node_ulr),
            announcement_link: Address::from_str(&channel_address, &announcement_tag).unwrap(),
            subscription_link: Address::default(),
            channel_address: channel_address,
        }
    }

    pub fn connect(&mut self) -> Result<String, &str> {
        let message_list = self
            .client
            .recv_messages_with_options(&self.announcement_link, ())
            .unwrap();

        let mut found_valid_msg = false;
        for tx in message_list.iter() {
            match tx.parse_header() {
                Ok(header) => {
                    if header.check_content_type(message::announce::TYPE) {
                        self.subscriber.unwrap_announcement(header.clone()).unwrap();
                        found_valid_msg = true;
                        break;
                    } else {
                        println!(
                            "Expected an announce message, found {}",
                            header.content_type()
                        );
                    }
                }
                Err(e) => {
                    println!("Parsing Error Header: {}", e);
                }
            };
        }
        if found_valid_msg {
            let subscribe_link = {
                let msg = self.subscriber.subscribe(&self.announcement_link).unwrap();
                self.client
                    .send_message_with_options(&msg, self.send_opt)
                    .unwrap();
                msg.link.clone()
            };

            self.subscription_link = subscribe_link;
            self.is_connected = true;
        } else {
            println!("No valid announce message found");
        }
        Ok(self.subscription_link.msgid.to_string())
    }

    pub fn disconnect(&mut self) -> Result<String, &str> {
        let unsubscribe_link = {
            let msg = self
                .subscriber
                .unsubscribe(&self.subscription_link)
                .unwrap();
            self.client
                .send_message_with_options(&msg, self.send_opt)
                .unwrap();
            msg.link.msgid
        };
        Ok(unsubscribe_link.to_string())
    }

    pub fn read_signed<T>(
        &mut self,
        signed_packet_tag: String,
        change_key_tag: Option<String>,
    ) -> Fallible<Vec<(Option<T>, Option<T>)>>
    where
        T: DeserializeOwned,
    {
        if change_key_tag.is_some() {
            self.update_change_key(change_key_tag.unwrap()).unwrap();
        }

        let mut response: Vec<(Option<T>, Option<T>)> = Vec::new();

        if self.is_connected {
            let link = Address::from_str(&self.channel_address, &signed_packet_tag).unwrap();

            let message_list = self.client.recv_messages_with_options(&link, ()).unwrap();
            for tx in message_list.iter() {
                match tx.parse_header() {
                    Ok(header) => {
                        if header.check_content_type(message::signed_packet::TYPE) {
                            match self.subscriber.unwrap_signed_packet(header.clone()) {
                                Ok((unwrapped_public, unwrapped_masked)) => {
                                    response.push((
                                        Payload::unwrap_data(&unwrapped_public)?,
                                        Payload::unwrap_data(&unwrapped_masked)?,
                                    ));
                                }
                                Err(e) => println!("Signed Packet Error: {}", e),
                            }
                        } else {
                            println!("Expected a signed message, found {}", header.content_type());
                        }
                    }
                    Err(e) => {
                        println!("Parsing Error Header: {}", e);
                    }
                };
            }
        } else {
            println!("Channel not connected");
        }

        Ok(response)
    }

    pub fn read_tagged<T>(
        &mut self,
        tagged_packet_tag: String,
    ) -> Fallible<Vec<(Option<T>, Option<T>)>>
    where
        T: DeserializeOwned,
    {
        let mut response: Vec<(Option<T>, Option<T>)> = Vec::new();

        if self.is_connected {
            let link = Address::from_str(&self.channel_address, &tagged_packet_tag).unwrap();

            let message_list = self.client.recv_messages_with_options(&link, ()).unwrap();

            for tx in message_list.iter() {
                match tx.parse_header() {
                    Ok(header) => {
                        if header.check_content_type(message::tagged_packet::TYPE) {
                            match self.subscriber.unwrap_tagged_packet(header.clone()) {
                                Ok((unwrapped_public, unwrapped_masked)) => {
                                    response.push((
                                        Payload::unwrap_data(&unwrapped_public)?,
                                        Payload::unwrap_data(&unwrapped_masked)?,
                                    ));
                                }
                                Err(e) => println!("Tagged Packet Error: {}", e),
                            }
                        } else {
                            println!("Expected a tagged message, found {}", header.content_type());
                        }
                    }
                    Err(e) => {
                        println!("Parsing Error Header: {}", e);
                    }
                };
            }
        } else {
            println!("Channel not connected");
        }

        Ok(response)
    }

    pub fn update_keyload(&mut self, keyload_tag: String) -> Fallible<()> {
        let keyload_link = Address::from_str(&self.channel_address, &keyload_tag).unwrap();

        if self.is_connected {
            let message_list = self
                .client
                .recv_messages_with_options(&keyload_link, ())
                .unwrap();

            for tx in message_list.iter() {
                let preparsed = match tx.parse_header() {
                    Ok(val) => Some(val),
                    Err(e) => {
                        println!("Parsing Error Header: {}", e);
                        None
                    }
                };
                match preparsed {
                    None => println!("Invalid message"),
                    Some(header) => {
                        if header.check_content_type(message::keyload::TYPE) {
                            match self.subscriber.unwrap_keyload(header.clone()) {
                                Ok(_) => {
                                    break;
                                }
                                Err(e) => println!("Keyload Packet Error: {}", e),
                            }
                        } else if header.check_content_type(message::change_key::TYPE) {
                            match self.subscriber.unwrap_change_key(header.clone()) {
                                Ok(_) => {
                                    break;
                                }
                                Err(e) => println!("Change Key Packet Error: {}", e),
                            }
                        } else {
                            println!(
                                "Expected a keyload message, found {}",
                                header.content_type()
                            );
                        }
                    }
                }
            }
        } else {
            println!("Channel not connected");
        }

        Ok(())
    }

    ///
    /// Update the change key
    ///
    pub fn update_change_key(&mut self, change_key_tag: String) -> Fallible<()> {
        let keyload_link = Address::from_str(&self.channel_address, &change_key_tag).unwrap();

        if self.is_connected {
            let message_list = self
                .client
                .recv_messages_with_options(&keyload_link, ())
                .unwrap();

            for tx in message_list.iter() {
                let preparsed = match tx.parse_header() {
                    Ok(val) => Some(val),
                    Err(e) => {
                        println!("Parsing Error Header: {}", e);
                        None
                    }
                };
                match preparsed {
                    None => println!("Invalid message"),
                    Some(header) => {
                        if header.check_content_type(message::change_key::TYPE) {
                            match self.subscriber.unwrap_change_key(header.clone()) {
                                Ok(_) => {
                                    break;
                                }
                                Err(e) => println!("Keyload Packet Error: {}", e),
                            }
                        } else {
                            println!(
                                "Expected a keyload message, found {}",
                                header.content_type()
                            );
                        }
                    }
                }
            }
        } else {
            println!("Channel not connected");
        }

        Ok(())
    }
}
