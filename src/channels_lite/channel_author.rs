#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::utils::payload::PacketPayload;
use crate::utils::response_write_signed::ResponseSigned;
use failure::Fallible;
use iota_lib_rs::prelude::iota_client;
use iota_streams::app::transport::tangle::client::SendTrytesOptions;
use iota_streams::app::transport::Transport;
use iota_streams::app_channels::{
    api::tangle::{Address, Author},
    message,
};
use std::string::ToString;

pub struct Channel {
    author: Author,
    client: iota_client::Client<'static>,
    send_opt: SendTrytesOptions,
    channel_address: String,
    announcement_link: Address,
    keyload_tag: String,
    mss_height: u32,
    /// Posible numbers of messages to sign before change the key
    remaining_signed_messages: u32,
}

impl Channel {
    pub fn new(seed: &str, node_ulr: &'static str) -> Channel {
        let mss_height = 3_u32;
        let author = Author::new(seed, mss_height as usize, true);

        let channel_address = author.channel_address().to_string();

        let mut send_opt = SendTrytesOptions::default();
        send_opt.min_weight_magnitude = 9;
        send_opt.local_pow = false;

        Self {
            author: author,
            client: iota_client::Client::new(node_ulr),
            send_opt: send_opt,
            channel_address: channel_address,
            announcement_link: Address::default(),
            keyload_tag: String::default(),
            mss_height: mss_height,
            remaining_signed_messages: 2_u32.pow(mss_height),
        }
    }

    pub fn open(&mut self) -> Result<(String, String), &str> {
        let announcement_message = self.author.announce().unwrap();
        self.client
            .send_message_with_options(&announcement_message, self.send_opt)
            .unwrap();
        let announcement_address: String = announcement_message.link.appinst.to_string();
        let announcement_tag: String = announcement_message.link.msgid.to_string();

        self.announcement_link =
            Address::from_str(&announcement_address, &announcement_tag).unwrap();

        Ok((self.channel_address.clone(), announcement_tag))
    }

    pub fn add_subscriber(&mut self, subscribe_tag: String) -> Result<String, &str> {
        let subscribe_link = Address::from_str(&self.channel_address, &subscribe_tag).unwrap();

        let message_list = self
            .client
            .recv_messages_with_options(&subscribe_link, ())
            .unwrap();
        for tx in message_list.iter() {
            match tx.parse_header() {
                Ok(header) => {
                    if header.check_content_type(message::subscribe::TYPE) {
                        match self.author.unwrap_subscribe(header.clone()) {
                            Ok(_) => {
                                break;
                            }
                            Err(e) => println!("Subscribe Packet Error: {}", e),
                        }
                    } else {
                        println!(
                            "Expected a subscription message, found {}",
                            header.content_type()
                        );
                    }
                }
                Err(e) => println!("Parsing Error Header: {}", e),
            }
        }

        self.keyload_tag = {
            let msg = self
                .author
                .share_keyload_for_everyone(&subscribe_link)
                .unwrap();
            self.client
                .send_message_with_options(&msg, self.send_opt)
                .unwrap();
            msg.link.msgid.to_string()
        };

        Ok(self.keyload_tag.clone())
    }

    pub fn write_signed<T>(&mut self, masked: bool, payload: T) -> Result<ResponseSigned, &str>
    where
        T: PacketPayload,
    {
        let change_key_tag = self.try_change_key(false).unwrap();

        let keyload_link = Address::from_str(&self.channel_address, &self.keyload_tag).unwrap();

        let signed_packet_link = {
            if masked {
                let msg = self
                    .author
                    .sign_packet(
                        &keyload_link,
                        &payload.public_data(),
                        &payload.masked_data(),
                    )
                    .unwrap();
                self.client
                    .send_message_with_options(&msg, self.send_opt)
                    .unwrap();
                msg.link.clone()
            } else {
                let msg = self
                    .author
                    .sign_packet(
                        &self.announcement_link,
                        &payload.public_data(),
                        &payload.masked_data(),
                    )
                    .unwrap();
                self.client
                    .send_message_with_options(&msg, self.send_opt)
                    .unwrap();
                msg.link.clone()
            }
        };

        Ok(ResponseSigned {
            signed_message_tag: signed_packet_link.msgid.to_string(),
            change_key_tag: change_key_tag,
        })
    }

    pub fn write_tagged<T>(&mut self, payload: T) -> Result<String, &str>
    where
        T: PacketPayload,
    {
        let keyload_link = Address::from_str(&self.channel_address, &self.keyload_tag).unwrap();

        let tagged_packet_link = {
            let msg = self
                .author
                .tag_packet(
                    &keyload_link,
                    &payload.public_data(),
                    &payload.masked_data(),
                )
                .unwrap();
            self.client
                .send_message_with_options(&msg, self.send_opt)
                .unwrap();
            msg.link.clone()
        };

        Ok(tagged_packet_link.msgid.to_string())
    }

    pub fn remove_subscriber(&mut self, unsubscribe_tag: String) -> Fallible<()> {
        let unsubscribe_link = Address::from_str(&self.channel_address, &unsubscribe_tag).unwrap();

        let message_list = self
            .client
            .recv_messages_with_options(&unsubscribe_link, ())
            .unwrap();
        for tx in message_list.iter() {
            match tx.parse_header() {
                Ok(header) => {
                    if header.check_content_type(message::unsubscribe::TYPE) {
                        match self.author.unwrap_unsubscribe(header.clone()) {
                            Ok(_) => {
                                break;
                            }
                            Err(e) => println!("Unsubscribe Packet Error: {}", e),
                        }
                    } else {
                        println!(
                            "Expected a unsubscription message, found {}",
                            header.content_type()
                        );
                    }
                }
                Err(e) => {
                    println!("Parsing Error Header: {}", e);
                }
            };
        }
        Ok(())
    }

    /// Try to do change key if not more signed key is available
    ///
    /// Return a Option with Message Id if the mss key is changed
    ///
    pub fn try_change_key(&mut self, force: bool) -> Fallible<Option<String>> {
        if self.remaining_signed_messages <= 0 || force == true {
            let msg = self.author.change_key(&self.announcement_link)?;
            self.remaining_signed_messages = 2_u32.pow(self.mss_height);
            self.client.send_message_with_options(&msg, self.send_opt)?;
            return Ok(Some(msg.link.msgid.to_string()));
        } else {
            self.remaining_signed_messages -= 1;
        }
        return Ok(None);
    }
}
