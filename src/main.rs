mod model;
mod view;

use actix_files::Files;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use crossbeam::channel::{unbounded, Receiver, Sender};
use env_logger::Env;
use log::{debug, error, info};
use model::{
    fetch::{DataPkt, Fetcher},
    init,
};
use rustls::{pki_types::PrivateKeyDer, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, thread};
use tokio::runtime::Builder;
use view::client::{getfeed, getforcefeed, getfull, gethome, Controller};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CmdVars {
    #[arg(short, long, value_name = "FILE")]
    config_file: Option<String>,
    #[arg(short, long, default_value_t = 7878)]
    port: u16,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: Option<u8>,
    #[arg(short = 'k', long, value_name = "CERT_BASE_DIR")]
    certificate: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = CmdVars::parse();
    let port = cli.port;

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

    // run downloader
    thread::spawn(move || {
        let downman = Arc::new(Fetcher::new(serverbag));
        loop {
            if let Ok(name) = model_rx.recv() {
                let downref = Arc::clone(&downman);
                if let DataPkt::Request(s, t) = name {
                    model_runtime.spawn(async move {
                        downref.get(DataPkt::Request(s, t)).await;
                    });
                } else if let DataPkt::ForceRequest(s, t) = name {
                    model_runtime.spawn(async move {
                        downref.forceget(DataPkt::ForceRequest(s, t)).await;
                    });
                }
            }
        }
    });

    // setup client
    let client = clientbag.clients.clone();
    let mut templatedir = clientbag.templatedir.clone();
    templatedir.push("static");
    let templatedir = templatedir.to_str().unwrap().to_string();
    let controller = web::Data::new(Controller::new(clientbag, model_tx));

    //get rustconfig
    let config = load_rustls_config(cli.certificate);
    match config {
        Some(config) => {
            HttpServer::new(move || {
                App::new()
                    .app_data(controller.clone())
                    .service(Files::new("/static", &templatedir))
                    .service(gethome)
                    .service(getfull)
                    .service(getfeed)
                    .service(getforcefeed)
            })
            .bind_rustls_0_23(("0.0.0.0", port), config)?
            .workers(client)
            .run()
            .await
        }
        None => {
            HttpServer::new(move || {
                App::new()
                    .app_data(controller.clone())
                    .service(Files::new("/static", &templatedir))
                    .service(gethome)
                    .service(getfull)
                    .service(getfeed)
                    .service(getforcefeed)
            })
            .bind(("0.0.0.0", port))?
            .workers(client)
            .run()
            .await
        }
    }
}

fn load_rustls_config(base_dir: Option<String>) -> Option<rustls::ServerConfig> {
    match base_dir {
        Some(dir) => {
            rustls::crypto::aws_lc_rs::default_provider()
                .install_default()
                .unwrap();
            // init server config builder with safe defaults
            let config = ServerConfig::builder().with_no_client_auth();

            let base = PathBuf::from(dir);
            let mut cert_loc = base.clone();
            cert_loc.push("cert.pem");
            let mut key_loc = base.clone();
            key_loc.push("key.pem");
            // load TLS key/cert files
            let cert_file = &mut BufReader::new(File::open(cert_loc).unwrap());
            let key_file = &mut BufReader::new(File::open(key_loc).unwrap());

            // convert files to key/cert objects
            let cert_chain = certs(cert_file).collect::<Result<Vec<_>, _>>().unwrap();
            let mut keys = pkcs8_private_keys(key_file)
                .map(|key| key.map(PrivateKeyDer::Pkcs8))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            // exit if no keys could be parsed
            if keys.is_empty() {
                error!("Could not locate PKCS 8 private keys.");
                std::process::exit(-1);
            }
            Some(config.with_single_cert(cert_chain, keys.remove(0)).unwrap())
        }
        None => {
            debug!("No base dir defined, won't be using tls");
            None
        }
    }
}
