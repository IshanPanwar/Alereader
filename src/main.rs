mod model;
mod view;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use crossbeam::channel::{unbounded, Receiver, Sender};
use env_logger::Env;
use log::info;
use model::{
    fetch::{DataPkt, Fetcher},
    init,
};
use std::{sync::Arc, thread};
use tokio::runtime::Builder;
use view::client::{getfeed, getforcefeed, getfull, gethome, getstatic, Controller};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CmdVars {
    #[arg(short, long, value_name = "FILE")]
    config_file: Option<String>,
    #[arg(short, long, default_value_t = 7878)]
    port: u16,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: Option<u8>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = CmdVars::parse();

    match cli.debug {
        Some(0) => env_logger::Builder::from_env(Env::default().default_filter_or("off")).init(),
        Some(1) => env_logger::Builder::from_env(Env::default().default_filter_or("error")).init(),
        Some(2) => env_logger::Builder::from_env(Env::default().default_filter_or("info")).init(),
        Some(3) => env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init(),
        _ => env_logger::Builder::from_env(Env::default().default_filter_or("off")).init(),
    }

    info!("Server init");

    let (clientbag, serverbag) = init::init(cli.config_file);

    let model_runtime = Builder::new_multi_thread()
        .worker_threads(serverbag.download.clone())
        .thread_name("alereader-downloader-runtime")
        .enable_all()
        .build()
        .unwrap();
    let (model_tx, model_rx): (Sender<DataPkt>, Receiver<DataPkt>) = unbounded();

    // vars
    let port = cli.port;

    // run downloader
    thread::spawn(move || {
        let downman = Arc::new(Fetcher::new(serverbag));
        loop {
            if let Ok(name) = model_rx.recv() {
                let downref = Arc::clone(&downman);
                model_runtime.spawn(async move {
                    downref.get(name).await;
                });
            }
        }
    });

    // run client side
    let client = clientbag.clients.clone();
    let controller = web::Data::new(Controller::new(clientbag, model_tx));
    HttpServer::new(move || {
        App::new()
            .app_data(controller.clone())
            .service(getstatic)
            .service(gethome)
            .service(getfull)
            .service(getfeed)
            .service(getforcefeed)
    })
    .bind(("0.0.0.0", port.clone()))
    .unwrap()
    .workers(client)
    .run()
    .await
}
