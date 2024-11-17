use atom_syndication::Feed;
use log::{debug, error};
use minijinja::{context, Environment, Value};
use rss::Channel;
use std::{collections::HashMap, fs, fs::read_to_string, path::PathBuf, process};

pub struct View {
    env: Environment<'static>,
}

impl View {
    pub fn new(templatedir: PathBuf) -> Self {
        //making env
        let mut env = Environment::new();
        let mut homefile = templatedir.clone();
        homefile.push("home.html");
        let mut feedfile = templatedir.clone();
        feedfile.push("feed.html");
        let mut errorfile = templatedir.clone();
        errorfile.push("error.html");

        let homecontents = match read_to_string(homefile.clone()) {
            Ok(s) => s,
            Err(_) => {
                error!("Failed to read homefile!");
                process::exit(-1);
            }
        };
        let feedcontents = match read_to_string(feedfile.clone()) {
            Ok(s) => s,
            Err(_) => {
                error!("Failed to read feedfile!");
                process::exit(-1);
            }
        };
        let errorcontents = match read_to_string(errorfile.clone()) {
            Ok(s) => s,
            Err(_) => {
                error!("Failed to read errorfile!");
                process::exit(-1);
            }
        };

        match env.add_template_owned(String::from("home"), homecontents) {
            Ok(()) => debug!("home.html has been parsed!"),
            Err(_) => {
                error!("Failed to add homefile to collection!");
                process::exit(-1);
            }
        }
        match env.add_template_owned(String::from("feed"), feedcontents) {
            Ok(()) => debug!("feed.html has been parsed!"),
            Err(_) => {
                error!("Failed to add feedfile to collection!");
                process::exit(-1);
            }
        }
        match env.add_template_owned(String::from("error"), errorcontents) {
            Ok(()) => debug!("error.html has been parsed!"),
            Err(_) => {
                error!("Failed to add errorfile to collection!");
                process::exit(-1);
            }
        }

        Self { env }
    }
    pub async fn servefeed_rss(&self, data: Channel) -> String {
        let tmp = self.env.get_template("feed").unwrap();
        let ctx = Value::from_serialize(data);
        match tmp.render(ctx) {
            Ok(s) => return s,
            Err(_) => return self.serveerror(500).await,
        }
    }
    pub async fn servefeed_atom(&self, data: Feed) -> String {
        let tmp = self.env.get_template("feed").unwrap();
        let ctx = Value::from_serialize(data);
        match tmp.render(ctx) {
            Ok(s) => return s,
            Err(_) => return self.serveerror(500).await,
        }
    }
    pub async fn servehome(&self, url_data: &HashMap<String, Vec<String>>) -> String {
        let home = self.env.get_template("home").unwrap();
        let mut dat = Vec::new();
        for i in url_data.keys() {
            dat.push(context!(heading => i, names => url_data.get(i).unwrap()))
        }
        match home.render(context!(headings => dat)) {
            Ok(s) => return s,
            Err(_) => return self.serveerror(500).await,
        }
    }

    pub async fn serveerror(&self, val: u16) -> String {
        let tmp = self.env.get_template("error").unwrap();
        match tmp.render(context!(error => val)) {
            Ok(s) => return s,
            Err(_) => {
                error!("Unable to render error template! Shutting down");
                process::exit(-1);
            }
        }
    }
}

fn getfiles(dir: PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }

    return files;
}
