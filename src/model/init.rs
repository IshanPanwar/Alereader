use directories::ProjectDirs;
use log::{debug, error, info};
use regex::Regex;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf, process};

#[derive(Debug, Clone)]
pub struct ClientBag {
    pub templatedir: PathBuf,
    pub headings: HashMap<String, Vec<String>>,
    pub clients: usize,
}

#[derive(Debug)]
pub struct ServerBag {
    pub names: HashMap<String, String>,
    pub archive_lst: Vec<String>,
    pub archivedir: Option<PathBuf>,
    pub useragent: String,
    pub download: usize,
    pub cachesz: usize,
}

fn projfiles() -> (String, String) {
    // returns project dir info, (config file, template folder)
    let proj_dirs = ProjectDirs::from("com", "Sanvi", "Sanvi").unwrap();
    let config_dir = proj_dirs.config_local_dir();
    let data_dir = config_dir;
    //convert config_dir to pathbuf
    let mut config_file = config_dir.to_path_buf();
    config_file.push("sanvi.conf");
    (
        config_file.to_str().unwrap().to_owned(),
        data_dir.to_str().unwrap().to_owned(),
    )
}

pub fn init(file: Option<String>) -> (ClientBag, ServerBag) {
    //resolve file
    let file = match file {
        Some(s) => s,
        None => {
            info!("No file supplied! Looking for default");
            let (file, _) = projfiles();
            file
        }
    };
    //open file
    let contents = match read_to_string(file.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to read file {file}! Error: {e}");
            process::exit(-1);
        }
    };

    debug!("File {file} has been read");

    let lines: Vec<&str> = contents.split("[RssLinks]").collect();
    if lines.len() == 2 {
        let (templatedir, archivedir, useragent, download, cachesz, clients) =
            settings_maker(lines[0].to_string());
        let (headings, names, archive_lst) = links_maker(lines[1].to_string());
        (
            ClientBag {
                templatedir,
                headings,
                clients,
            },
            ServerBag {
                names,
                archive_lst,
                archivedir,
                useragent,
                download,
                cachesz,
            },
        )
    } else {
        error!("File formatting error, [RssLinks] is present either more than 1 times or not present at all");
        process::exit(-1);
    }
}

fn settings_maker(hay: String) -> (PathBuf, Option<PathBuf>, String, usize, usize, usize) {
    //defining regex patterns
    let template_rgx = Regex::new(r"\s*template\s*=\s*([^\s]+)\s*").unwrap();
    let archive_rgx = Regex::new(r"\s*archive\s*=\s*([^\s]+)\s*").unwrap();
    let ua_rgx = Regex::new(r"\s*useragent\s*=\s*\'([^\']+)\'").unwrap();
    let download_rgx = Regex::new(r"\s*download\s*=\s*([^\s]+)\s*").unwrap();
    let cache_rgx = Regex::new(r"\s*cache-limit\s*=\s*([^\s]+)\s*").unwrap();
    let clients_rgx = Regex::new(r"\s*clients\s*=\s*([^\s]+)\s*").unwrap();

    //find text
    let template = match template_rgx.captures(hay.as_str()) {
        Some(path) => PathBuf::from(path.get(1).unwrap().as_str()),
        None => {
            info!("Template location not found, using default");
            let (_, template_dir) = projfiles();
            PathBuf::from(template_dir)
        }
    };
    let archive = match archive_rgx.captures(hay.as_str()) {
        Some(path) => Some(PathBuf::from(path.get(1).unwrap().as_str())),
        None => {
            info!("Archive dir not found, will not be archiving");
            None
        }
    };
    let useragent = match ua_rgx.captures(hay.as_str()) {
        Some(path) => path.get(1).unwrap().as_str().to_string(),
        None => {
            info!("Useragent not found, using default");
            String::from(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.1",
            )
        }
    };
    let download = match download_rgx.captures(hay.as_str()) {
        Some(num) => match num.get(1).unwrap().as_str().parse::<usize>() {
            Ok(number) => number,
            Err(_) => {
                error!("failed to parse download, using defaults");
                3usize
            }
        },
        None => {
            info!("No download information found, using default");
            3usize
        }
    };
    let cachesz = match cache_rgx.captures(hay.as_str()) {
        Some(num) => match num.get(1).unwrap().as_str().parse::<usize>() {
            Ok(number) => number,
            Err(_) => {
                error!("failed to parse download, using defaults");
                10usize
            }
        },
        None => {
            info!("No download information found, using default");
            10usize
        }
    };
    let clients = match clients_rgx.captures(hay.as_str()) {
        Some(num) => match num.get(1).unwrap().as_str().parse::<usize>() {
            Ok(number) => number,
            Err(_) => {
                error!("Failed to parse download, using defaults");
                3usize
            }
        },
        None => {
            info!("No download information found, using default");
            3usize
        }
    };
    debug!("Settings parsed!");

    //return struct
    (template, archive, useragent, download, cachesz, clients)
}

fn links_maker(
    s: String,
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, String>,
    Vec<String>,
) {
    //basic data
    let mut headings = HashMap::new();
    let mut names = HashMap::new();
    let mut archive_lst = Vec::new();

    let rss_pat =
        Regex::new(r"^([^,]+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*(?:$|,\s*([A-Za-z]{1,2})$)").unwrap();
    //find regex
    let lines: Vec<_> = s.split('\n').filter(|x| !x.is_empty()).collect();
    for line in lines {
        debug!("Parsing line {line}");
        let capts = rss_pat.captures(line).unwrap();
        let heading = match capts.get(1) {
            Some(x) => x.as_str(),
            None => {
                error!("Error parsing line {line}! Could not parse heading");
                process::exit(-1);
            }
        };
        let name = match capts.get(2) {
            Some(x) => x.as_str(),
            None => {
                error!("Error parsing line {line}! Could not parse name");
                process::exit(-1);
            }
        };
        let link = match capts.get(3) {
            Some(x) => x.as_str(),
            None => {
                error!("Error parsing line {line}! Could not parse link");
                process::exit(-1);
            }
        };
        match capts.get(4) {
            Some(mat) => {
                let ma: Vec<char> = mat.as_str().to_string().chars().collect();
                if ma[0] == 'Y' || ma[0] == 'y' {
                    archive_lst.push(name.to_string());
                }
            }
            None => {
                info!("Opts not set for {line}");
            }
        }
        headings
            .entry(heading.to_string())
            .or_insert_with(Vec::new)
            .push(name.to_string());
        if names.contains_key(name) {
            error!("Names must be unique, there are two links with the name {name}");
            process::exit(-1);
        } else {
            names.insert(name.to_string(), link.to_string());
        }
    }
    debug!("RSS links parsed!");
    (headings, names, archive_lst)
}
