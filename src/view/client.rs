use crate::model::{fetch::DataPkt, init::ClientBag};
use crate::view::compose::View;
use actix_web::{
    get,
    http::header::ContentType,
    web::{Data, Path},
    HttpResponse,
};
use crossbeam::channel::{unbounded, Receiver, Sender};
use log::info;
use std::collections::HashMap;

pub struct Controller {
    pub view: View,
    store: ClientBag,
    pub view_tx: Sender<DataPkt>,
}

impl Controller {
    pub fn new(store: ClientBag, view_tx: Sender<DataPkt>) -> Self {
        let view = View::new(store.templatedir.clone());

        Self {
            view,
            store,
            view_tx,
        }
    }
    pub fn headings_ref(&self) -> &HashMap<String, Vec<String>> {
        return &self.store.headings;
    }
}

#[get("/")]
pub async fn gethome(data: Data<Controller>) -> HttpResponse {
    info!("Request for /");
    return HttpResponse::Ok().body(data.view.servehome(&data.headings_ref()).await);
}

#[get("/force/{headings}/{name}/")]
pub async fn getforcefeed(data: Data<Controller>, name: Path<(String, String)>) -> HttpResponse {
    info!("Request for /force/{}/{}", name.0.clone(), name.1.clone());
    let (req_tx, req_rx): (Sender<DataPkt>, Receiver<DataPkt>) = unbounded();
    data.view_tx
        .send(DataPkt::Request(name.1.clone(), req_tx))
        .unwrap();
    if let Ok(result) = req_rx.recv() {
        let ret = match result {
            DataPkt::Error(val) => data.view.serveerror(val).await,
            DataPkt::Channel(chan) => data.view.servefeed_rss(chan).await,
            DataPkt::Feed(feed) => data.view.servefeed_atom(feed).await,
            _ => data.view.serveerror(404).await,
        };
        return HttpResponse::Ok().body(ret);
    } else {
        return HttpResponse::Ok().body(data.view.serveerror(500).await);
    }
}

#[get("/{heading}/{name}/")]
pub async fn getfeed(data: Data<Controller>, name: Path<(String, String)>) -> HttpResponse {
    info!("Request for /{}/{}", name.0.clone(), name.1.clone());
    let (req_tx, req_rx): (Sender<DataPkt>, Receiver<DataPkt>) = unbounded();
    data.view_tx
        .send(DataPkt::Request(name.1.clone(), req_tx))
        .unwrap();
    if let Ok(result) = req_rx.recv() {
        let ret = match result {
            DataPkt::Error(val) => data.view.serveerror(val).await,
            DataPkt::Channel(chan) => data.view.servefeed_rss(chan).await,
            DataPkt::Feed(feed) => data.view.servefeed_atom(feed).await,
            _ => data.view.serveerror(404).await,
        };
        return HttpResponse::Ok().body(ret);
    } else {
        return HttpResponse::Ok().body(data.view.serveerror(500).await);
    }
}

#[get("/{heading}/")]
pub async fn getfull(data: Data<Controller>, name: Path<String>) -> HttpResponse {
    info!("Request for /{}/", name.clone());
    let headings = data.headings_ref();
    if let Some(list) = headings.get(&name.to_string()) {
        let mut composite = String::new();
        let (req_tx, req_rx): (Sender<DataPkt>, Receiver<DataPkt>) = unbounded();

        for name in list {
            data.view_tx
                .send(DataPkt::Request(name.to_string(), req_tx.clone()))
                .unwrap();

            if let Ok(result) = req_rx.recv() {
                let ret = match result {
                    DataPkt::Error(val) => data.view.serveerror(val).await,
                    DataPkt::Channel(chan) => data.view.servefeed_rss(chan).await,
                    DataPkt::Feed(feed) => data.view.servefeed_atom(feed).await,
                    _ => data.view.serveerror(404).await,
                };
                composite.push_str(&ret);
            } else {
                composite.push_str(&data.view.serveerror(500).await)
            }
        }

        return HttpResponse::Ok().body(composite);
    } else {
        return HttpResponse::Ok().body(data.view.serveerror(404).await);
    }
}
