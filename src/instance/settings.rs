use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use std::{collections::HashMap, fs::File, io::Read};

#[derive(Debug)]
pub struct Vars {
    pub templateloc: String,
    pub archive: Option<String>, // If none, no archiving, else, provide dir
    pub ua: Option<String>,      //useragent when site is a bitch
    pub parallel: u16,           //default = 5
    pub cache: u16,              // default = 50, how many pages to cache
}

#[derive(Debug)]
pub struct Settings {
    pub vars: Vars,
    cache: HashMap<String, String>,
    links: HashMap<String, Vec<String>>, //heading, link
    build: HashMap<String, Vec<String>>, //link, elements which need to be used
}

impl Settings {
    pub fn read(s_file: &str) -> Result<Self> {
        let mut file = File::open(s_file).expect("{s_file} does not exist");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Error while reading {s_file}"); // file ops over
        let split_res: Vec<&str> = contents.split("[RssLinks]").collect();

        let mut links: HashMap<String, Vec<String>> = HashMap::new();
        let mut blacklist: HashMap<String, Vec<String>> = HashMap::new();

        if split_res.len() != 2 {
            return Err(anyhow!("Settings file formatting error!"));
        } else {
            let vars = match Vars::read(split_res[0]) {
                Ok(e) => e,
                Err(e) => return Err(anyhow!("Error while reading vars:\n{e}")),
            };
            for i in split_res[1].split('\n').filter(|x| !x.is_empty()) {
                let record: Vec<String> = i
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .map(|x| x.split_whitespace().collect::<Vec<_>>().join(" "))
                    .collect();
                if record.len() != 3 {
                    return Err(anyhow!("RssLinks is ill-formatted. Record:\n{:?}", i));
                } else {
                    if let Some(x) = links.get_mut(record[0].as_str()) {
                        x.push(record[1].clone());
                    } else {
                        links.insert(record[0].clone(), vec![record[1].clone()]);
                    }
                    let black = record[2]
                        .split(" ")
                        .map(String::from)
                        .filter(|x| !x.is_empty())
                        .collect();
                    blacklist.insert(record[1].to_string(), black);
                }
            }
            let cache: HashMap<String, String> = HashMap::with_capacity(vars.cache.into());
            return Ok(Self {
                cache,
                vars,
                links,
                blacklist,
            });
        }
    }
    pub fn get_links(&self) -> Vec<String> {
        let vals = self.links.values().flatten().collect::<Vec<&String>>();
        let mut ret: Vec<String> = Vec::with_capacity(vals.len());
        for val in vals {
            ret.push(val.to_string())
        }
        ret
    }
    pub fn get_headings(&self) -> Vec<String> {
        let vals = self.links.keys().collect::<Vec<&String>>();
        let mut ret: Vec<String> = Vec::with_capacity(self.links.len());
        for val in vals {
            ret.push(val.to_string())
        }
        ret
    }
    pub fn cache(&mut self, url: String, channel: String) {
        println!("Inserting into cache");
        if !self.cache.contains_key(&url) {
            if self.cache.len() > self.cachesize() {
                let key = self
                    .cache
                    .keys()
                    .take(self.cache.len().saturating_sub(self.cachesize()))
                    .next()
                    .unwrap()
                    .clone();
                self.cache.remove_entry(&key);
            }
            self.cache.insert(url, channel);
        }
    }
    pub fn get_blacklist(&self, link: &String) -> &Vec<String> {
        self.blacklist.get(link).unwrap()
    }
    pub fn cachesize(&self) -> usize {
        self.vars.cache.into()
    }
    pub fn parallelization(&self) -> usize {
        self.vars.parallel.into()
    }
    pub fn useragent(&self) -> Option<String> {
        match self.vars.ua {
            Some(ref r) => Some(r.clone()),
            None => None,
        }
    }
}

impl Vars {
    pub fn read(s: &str) -> Result<Self> {
        let proj = ProjectDirs::from("com", "Ishan", "RssReader").unwrap();
        let mut dir = proj.config_dir().to_owned();
        dir.push("templates");

        let mut vars = Self {
            templateloc: dir.to_str().unwrap().to_string(),
            archive: None,
            parallel: 5,
            ua: None,
            cache: 50,
        };
        for var in s
            .split("\n")
            .map(|record| {
                record
                    .split("=")
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<&str>>()
            })
            .filter(|x| !x.is_empty())
        {
            if var.len() != 2 {
                return Err(anyhow!("Vars section not correctly set. Var:\n{:#?}", var));
            } else {
                match var[0]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str()
                {
                    "templateloc" => {
                        vars.templateloc = var[1]
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split('"')
                            .filter(|x| !x.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                    "ua" => {
                        vars.ua = Some(
                            var[1]
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .split('"')
                                .filter(|x| !x.is_empty())
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    }
                    "archive" => {
                        vars.archive = Some(
                            var[1]
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .split('"')
                                .filter(|x| !x.is_empty())
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    }
                    "parallel" => {
                        vars.parallel = match var[1]
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .parse::<u16>()
                        {
                            Ok(e) => e,
                            Err(_) => {
                                return Err(anyhow!("Failed to parse degree of parallelization!"))
                            }
                        }
                    }
                    "cache" => {
                        vars.cache = match var[1]
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .parse::<u16>()
                        {
                            Ok(e) => e,
                            Err(_) => return Err(anyhow!("Failed to parse size of cache!")),
                        }
                    }
                    &_ => return Err(anyhow!("Unknown var defined! ({:#?})", var)),
                }
            }
        }
        Ok(vars)
    }
}
