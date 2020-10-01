//!
//! Channel Subscriber
//!
use super::Network;
use crate::utils::{payload::json::Payload, random_seed};
use core::cell::RefCell;
use iota::client as iota_client;
use iota_streams::app::transport::tangle::{
    client::{RecvOptions, SendTrytesOptions},
    PAYLOAD_BYTES,
};
use iota_streams::app::transport::Transport;
use iota_streams::app_channels::{
    api::{
        tangle::{Address, Subscriber},
        SequencingState,
    },
    message,
};

use iota_streams::core::prelude::{Rc, String};

use anyhow::Result;

///
/// Channel subscriber
///
pub struct Channel {
    pub subscriber: Subscriber<&'static iota_client::Client>,
    is_connected: bool,
    send_opt: SendTrytesOptions,
    announcement_link: Address,
    subscription_link: Address,
    channel_address: String,
}

impl Channel {
    ///
    /// Initialize the subscriber
    ///
    pub fn new(
        node: Network,
        channel_address: String,
        announcement_tag: String,
        seed_option: Option<String>,
    ) -> Channel {
        let seed = match seed_option {
            Some(seed) => seed,
            None => random_seed::new(),
        };
        iota_client::Client::add_node(node.as_string()).unwrap();
        let subscriber = Subscriber::new(
            &seed,
            "utf-8",
            PAYLOAD_BYTES,
            Rc::new(RefCell::new(iota_client::Client::get())),
        );

        Self {
            subscriber: subscriber,
            is_connected: false,
            send_opt: node.send_options(),
            announcement_link: Address::from_str(&channel_address, &announcement_tag).unwrap(),
            subscription_link: Address::default(),
            channel_address: channel_address,
        }
    }

    ///
    /// Connect
    ///
    pub fn connect(&mut self) -> Result<String> {
        self.subscriber
            .receive_announcement(&self.announcement_link)?;

        let subscribe_link = {
            let msg = self.subscriber.send_subscribe(&self.announcement_link)?;
            msg
        };

        self.subscription_link = subscribe_link;
        self.is_connected = true;
        Ok(self.subscription_link.msgid.to_string())
    }

    /*
    ///
    /// Disconnect
    ///
    pub fn disconnect(&mut self) -> Result<String> {
        let unsubscribe_link = {
            let msg = self.subscriber.unsubscribe(&self.subscription_link)?;
            iota_client::Client::get().send_message_with_options(&msg, self.send_opt)?;
            msg.link.msgid
        };
        Ok(unsubscribe_link.to_string())
    }*/

    ///
    /// Read signed packet
    ///
    pub fn read_signed(
        &mut self,
        signed_packet_tag: String,
    ) -> Result<Vec<(Option<String>, Option<String>)>> {
        let mut response: Vec<(Option<String>, Option<String>)> = Vec::new();
        let link = Address::from_str(&self.channel_address, &signed_packet_tag).unwrap();

        if self.is_connected {
            match self.subscriber.receive_signed_packet(&link.clone()) {
                Ok((_signer, unwrapped_public, unwrapped_masked)) => {
                    response.push((
                        Payload::unwrap_data(&String::from_utf8(unwrapped_public.0).unwrap())
                            .unwrap(),
                        Payload::unwrap_data(&String::from_utf8(unwrapped_masked.0).unwrap())
                            .unwrap(),
                    ));
                }
                Err(e) => println!("Signed Packet Error: {}", e),
            }
        } else {
            println!("Channel not connected");
        }

        Ok(response)
    }

    ///
    /// Read tagged packet
    ///
    pub fn read_tagged(
        &mut self,
        tagged_packet_tag: String,
    ) -> Result<Vec<(Option<String>, Option<String>)>> {
        let mut response: Vec<(Option<String>, Option<String>)> = Vec::new();

        if self.is_connected {
            let link = Address::from_str(&self.channel_address, &tagged_packet_tag).unwrap();

            match self.subscriber.receive_tagged_packet(&link.clone()) {
                Ok((unwrapped_public, unwrapped_masked)) => {
                    response.push((
                        Payload::unwrap_data(&String::from_utf8(unwrapped_public.0).unwrap())
                            .unwrap(),
                        Payload::unwrap_data(&String::from_utf8(unwrapped_masked.0).unwrap())
                            .unwrap(),
                    ));
                }
                Err(e) => println!("Tagged Packet Error: {}", e),
            }
        } else {
            println!("Channel not connected");
        }

        Ok(response)
    }

    ///
    /// Update keyload
    ///
    pub fn update_keyload(&mut self, keyload_tag: String) -> Result<()> {
        let keyload_link = Address::from_str(&self.channel_address, &keyload_tag).unwrap();

        if self.is_connected {
            self.subscriber.receive_keyload(&keyload_link.clone());
        } else {
            println!("Channel not connected");
        }

        Ok(())
    }

    ///
    /// Generates the next message in the channels
    ///
    pub fn get_next_message(&mut self) -> Vec<Option<String>> {
        let mut exists = true;

        let mut tags: Vec<Option<String>> = vec![];

        while exists {
            let msgs = self.subscriber.fetch_next_msgs();
            exists = false;

            for msg in msgs {
                println!("Message exists at {}... ", &msg.link.msgid);
                tags.push(Some(msg.link.msgid.to_string()));
                exists = true;
            }

            if !exists {
                println!("No more messages in sequence.");
            }
        }
        tags
    }
}
