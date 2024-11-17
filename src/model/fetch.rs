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
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    process,
};
use tokio::{
    fs,
    fs::{try_exists, File},
    io::AsyncWriteExt,
};

pub enum DataPkt {
    Error(u16),
    Request(String, Sender<DataPkt>),
    ForceRequest(String, Sender<DataPkt>),
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
            DataPkt::ForceRequest(name, tx) => (name, tx),
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
                            self.archive(&txt, &name).await;
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
                                    self.archive(&txt, &name).await;
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
                }
            }
        }
    }
    async fn archive(&self, data: &str, name: &str) {
        if self.store.archive_lst.contains(&name.to_string()) {
            if let Some(mut dir) = self.store.archivedir.clone() {
                debug!("Beginning archive process");
                let mut name = name.to_string();
                name.push_str(".xml");
                dir.push(name);
                //check if filename exits
                if try_exists(dir.clone()).await.unwrap() {
                    match fs::read_to_string(dir.clone()).await {
                        Ok(s) => {
                            if calculate_hash(&s) != calculate_hash(&data.to_string()) {
                                match File::create(dir.clone()).await {
                                    Ok(mut s) => s.write_all(data.as_bytes()).await.unwrap(),
                                    Err(_) => error!("Unable to write to file {:?}", dir),
                                }
                            } else {
                                debug!("Data already archived")
                            }
                        }
                        Err(_) => error!("Unable to read file {:?}", dir),
                    };
                } else {
                    match File::create_new(dir.clone()).await {
                        Ok(mut s) => s.write_all(data.as_bytes()).await.unwrap(),
                        Err(_) => error!("Unable to create file {:?}", dir),
                    }
                }
            } else {
                error!("archivedir not defined, Not archiving");
            }
            debug!("Archiving complete");
        }
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
