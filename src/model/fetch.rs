use crate::init::ServerBag;
use atom_syndication::Feed;
use crossbeam::channel::Sender;
use log::{debug, error, info};
use quick_cache::{
    sync::{Cache, DefaultLifecycle},
    DefaultHashBuilder, OptionsBuilder, UnitWeighter,
};
use reqwest::{
    header::{HeaderMap, USER_AGENT},
    Client,
};
use rss::Channel;
use std::process;

pub enum DataPkt {
    Error(u16),
    Request(String, Sender<DataPkt>),
    Channel(Channel),
    Feed(Feed),
}

pub struct Fetcher {
    store: ServerBag,
    cache: Cache<String, String>,
}

impl Fetcher {
    pub fn new(store: ServerBag) -> Self {
        let lim = {
            if store.cachesz <= u64::MAX as usize {
                store.cachesz.clone() as u64
            } else {
                info!("Unable to convert cache size to u64, using 100u64");
                100u64
            }
        };
        let cache = Cache::<String, String>::with_options(
            OptionsBuilder::new()
                .weight_capacity(lim)
                .estimated_items_capacity(store.cachesz.clone())
                .build()
                .unwrap(),
            UnitWeighter,
            DefaultHashBuilder::default(),
            DefaultLifecycle::default(),
        );
        Self { store, cache }
    }
    pub async fn forceget(&self, data: DataPkt) {
        let (name, tx) = match data {
            DataPkt::Request(name, tx) => (name, tx),
            _ => {
                error!("Illegal request received! Shutting down");
                process::exit(-1);
            }
        };

        info!("Searching for link of {name}");

        match self.store.names.get(&name) {
            Some(s) => {
                let client = Client::new();
                let mut headers = HeaderMap::new();
                headers.insert(USER_AGENT, self.store.useragent.clone().parse().unwrap());

                match client.get(s).headers(headers).send().await {
                    Ok(resp) => match resp.text().await {
                        Ok(txt) => {
                            if let Ok(channel) = Channel::read_from(&txt.as_bytes()[..]) {
                                tx.send(DataPkt::Channel(channel)).unwrap();
                            } else if let Ok(feed) = Feed::read_from(&txt.as_bytes()[..]) {
                                tx.send(DataPkt::Feed(feed)).unwrap();
                            }
                            self.cache.insert(name, txt);
                        }
                        Err(_) => {
                            error!("Failed to parse response from {s}");
                            tx.send(DataPkt::Error(502)).unwrap();
                        }
                    },
                    Err(_) => {
                        error!("Failed to fetch from {s}");
                        tx.send(DataPkt::Error(502)).unwrap();
                    }
                }
            }
            None => tx.send(DataPkt::Error(502)).unwrap(),
        } //inner match end
    }
    pub async fn get(&self, data: DataPkt) {
        let (name, tx) = match data {
            DataPkt::Request(name, tx) => (name, tx),
            _ => {
                error!("Illegal request received! Shutting down");
                process::exit(-1);
            }
        };

        info!("Searching for site named {name}");

        match self.cache.get(&name) {
            Some(s) => {
                debug!("{name} found! returning value");
                if let Ok(channel) = Channel::read_from(&s.as_bytes()[..]) {
                    tx.send(DataPkt::Channel(channel)).unwrap();
                } else if let Ok(feed) = Feed::read_from(&s.as_bytes()[..]) {
                    tx.send(DataPkt::Feed(feed)).unwrap();
                }
            }
            None => {
                debug!("{name} not found! fetching from web!");
                match self.store.names.get(&name) {
                    Some(s) => {
                        let client = Client::new();
                        let mut headers = HeaderMap::new();
                        headers.insert(USER_AGENT, self.store.useragent.clone().parse().unwrap());

                        match client.get(s).headers(headers).send().await {
                            Ok(resp) => match resp.text().await {
                                Ok(txt) => {
                                    if let Ok(channel) = Channel::read_from(&txt.as_bytes()[..]) {
                                        tx.send(DataPkt::Channel(channel)).unwrap();
                                    } else if let Ok(feed) = Feed::read_from(&txt.as_bytes()[..]) {
                                        tx.send(DataPkt::Feed(feed)).unwrap();
                                    }
                                    self.cache.insert(name, txt);
                                }
                                Err(_) => {
                                    error!("Failed to parse response from {s}");
                                    tx.send(DataPkt::Error(502)).unwrap();
                                }
                            },
                            Err(_) => {
                                error!("Failed to fetch from {s}");
                                tx.send(DataPkt::Error(502)).unwrap();
                            }
                        }
                    }
                    None => tx.send(DataPkt::Error(502)).unwrap(),
                } //inner match end
            } //none end
        } //outer match end
    } //function end
    async fn archive(&self, data: &str) {
        todo!()
    }
}
