use crate::settings::Settings;
use anyhow::{anyhow, Result};
use futures::StreamExt;
use reqwest::{header::USER_AGENT, Client};
use rss::Channel;
use std::io::BufRead;

pub async fn buildcache(app: &mut Settings) -> Result<()> {
    let links = app.get_links();
    let parallel = app.parallelization();
    let ua = app.useragent();
    println!("us: {:#?}", ua);

    let fetches = futures::stream::iter(links.into_iter().map(|path| {
        let ua = match ua {
            Some(ref r) => Some(r.clone()),
            None => None,
        };
        async move {
            let mut client = Client::new().get(&path);

            if let Some(uastr) = ua {
                client = client.header(USER_AGENT, uastr)
            }

            match client.send().await {
                Ok(response) => match response.bytes().await {
                    Ok(b) => return createfeed(&path, &b[..]).await,
                    Err(e) => (
                        path.to_string(),
                        Err(anyhow!("Failed to parse response: {:?}", e)),
                    ),
                },
                Err(e) => (
                    path.to_string(),
                    Err(anyhow!("Failed to get response: {:?}", e)),
                ),
            }
        }
    }))
    .buffer_unordered(parallel)
    .collect::<Vec<_>>()
    .await;

    /*for fetch in fetches {
        if fetch.1.is_none() {
            println!("fetch results: {:#?}", fetch);
        }
    }*/

    /*let res = fetches.into_iter().map(|fetch| {
        if let Some(tup) = fetch {
            println!("async section cache start!");
            app.cache(tup.0, tup.1.to_string());
        }
    });
    println!("res: {:#?}", res);*/
    Ok(())
}

pub async fn createfeed<R: BufRead>(url: &str, bytes: R) -> (String, Result<String>) {
    todo!()
}
