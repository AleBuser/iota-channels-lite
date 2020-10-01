//!
//! Channel author
//!
use super::Network;
use crate::utils::{payload::PacketPayload, random_seed};
use anyhow::{bail, Result};
use core::cell::RefCell;
use iota::client as iota_client;
use iota_streams::app::transport::tangle::{
    client::{RecvOptions, SendTrytesOptions},
    PAYLOAD_BYTES,
};
use iota_streams::app_channels::{
    api::tangle::{Address, Author},
    message,
};
use iota_streams::{
    app::transport::Transport,
    core::prelude::{Rc, String},
};
use std::string::ToString;

///
/// Channel
///
pub struct Channel {
    author: Author<&'static iota_client::Client>,
    send_opt: SendTrytesOptions,
    channel_address: String,
    announcement_id: String,
    last_keyload_tag: String,
    previous_msg_tag: String,
}

impl Channel {
    ///
    /// Initialize the Channel
    ///
    pub fn new(node: Network, seed_option: Option<String>) -> Channel {
        let seed = match seed_option {
            Some(seed) => seed,
            None => random_seed::new(),
        };
        iota_client::Client::add_node(node.as_string()).unwrap();
        let author = Author::new(
            &seed,
            "utf-8",
            PAYLOAD_BYTES,
            false,
            Rc::new(RefCell::new(iota_client::Client::get())),
        );

        let channel_address = author.channel_address().unwrap().to_string();

        Self {
            author: author,
            send_opt: node.send_options(),
            channel_address: channel_address,
            announcement_id: String::default(),
            last_keyload_tag: String::default(),
            previous_msg_tag: String::default(),
        }
    }

    ///
    /// Open a channel
    ///
    pub fn open(&mut self) -> Result<(String, String)> {
        let announcement_message = self.author.send_announce()?;

        self.announcement_id = announcement_message.msgid.to_string();

        Ok((self.channel_address.clone(), self.announcement_id.clone()))
    }

    ///
    /// Add subscriber
    ///
    pub fn add_subscriber(&mut self, subscribe_tag: String) -> Result<String> {
        let subscribe_link = match Address::from_str(&self.channel_address, &subscribe_tag) {
            Ok(subscribe_link) => subscribe_link,
            Err(()) => bail!(
                "Failed to create Address from {}:{}",
                &self.channel_address,
                &subscribe_tag
            ),
        };

        let message_list = self.author.receive_subscribe(&subscribe_link)?;

        let announce_link =
            Address::from_str(&self.channel_address, &self.announcement_id).unwrap();

        self.last_keyload_tag = {
            let keyload = self.author.send_keyload_for_everyone(&announce_link)?;
            keyload.0.msgid.to_string()
        };

        Ok(self.last_keyload_tag.clone())
    }

    ///
    /// Write signed packet
    ///
    pub fn write_signed<T>(&mut self, payload: T) -> Result<String>
    where
        T: PacketPayload,
    {
        let signed_packet_link = {
            if self.previous_msg_tag == String::default() {
                let keyload_link =
                    Address::from_str(&self.channel_address, &self.last_keyload_tag).unwrap();
                let msg = self.author.send_signed_packet(
                    &keyload_link,
                    &payload.public_data(),
                    &payload.masked_data(),
                )?;
                let ret_link = msg.0;
                ret_link.clone()
            } else {
                let msg = self.author.send_signed_packet(
                    &Address::from_str(&self.channel_address, &self.previous_msg_tag).unwrap(),
                    &payload.public_data(),
                    &payload.masked_data(),
                )?;
                let ret_link = msg.0;
                ret_link.clone()
            }
        };

        self.previous_msg_tag = signed_packet_link.msgid.to_string().clone();

        Ok(signed_packet_link.msgid.to_string())
    }

    ///
    /// Write tagged packet
    ///
    pub fn write_tagged<T>(&mut self, payload: T) -> Result<String>
    where
        T: PacketPayload,
    {
        let _keyload_link =
            Address::from_str(&self.channel_address, &self.last_keyload_tag).unwrap();
        let tagged_packet_link = {
            if self.previous_msg_tag == String::default() {
                let keyload_link =
                    Address::from_str(&self.channel_address, &self.last_keyload_tag).unwrap();
                let msg = self.author.send_tagged_packet(
                    &keyload_link,
                    &payload.public_data(),
                    &payload.masked_data(),
                )?;
                let ret_link = msg.0;
                ret_link.clone()
            } else {
                let previous_msg_link =
                    Address::from_str(&self.channel_address, &self.previous_msg_tag).unwrap();
                let msg = self.author.send_tagged_packet(
                    &previous_msg_link,
                    &payload.public_data(),
                    &payload.masked_data(),
                )?;
                let ret_link = msg.0;
                ret_link.clone()
            }
        };

        Ok(tagged_packet_link.msgid.to_string())
    }
    /*
    ///
    /// Remove subscriber
    ///
    ///
    pub fn remove_subscriber(&mut self, unsubscribe_tag: String) -> Result<()> {
        let unsubscribe_link = Address::from_str(&self.channel_address, &unsubscribe_tag).unwrap();

        let message_list = iota_client::Client::get()
            .recv_messages_with_options(&unsubscribe_link, RecvOptions::default())?;
        for tx in message_list.iter() {
            let header = tx.parse_header()?;
            if header.check_content_type(message::UNSUBSCRIBE) {
                match self.author.unsubscribe(header.clone()) {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => println!("Unsubscribe Packet Error: {}", e),
                }
            }
        }
        Ok(())
    }
    */
}
