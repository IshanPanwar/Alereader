mod app;
mod settings;

use app::buildcache;
use settings::{Settings, Vars};
use tokio;

#[tokio::main]
async fn main() {
    let mut instance =
        Settings::read("/home/sally/Documents/Hobby/Programs/Rssreader/rssreader/rss.settings")
            .expect("Failure to create app");
    buildcache(&mut instance).await;
    //print!("{:#?}", instance);
}
