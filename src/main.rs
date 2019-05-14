extern crate num_cpus;
extern crate regex;
use regex::Regex;
extern crate whois_rust;


use std::{
    fs::OpenOptions,
    io::{Read, Write},
    process,
    sync::Arc,
    thread,
};
use whois_rust::idna;
use whois_rust::{WhoIs, WhoIsLookupOptions};
fn main() {
    let cpus = num_cpus::get();

    let mut alive_string = String::new();
    match OpenOptions::new()
        .read(true)
        .open("alive.txt")
        .unwrap()
        .read_to_string(&mut alive_string)
    {
        Err(_) => eprintln!("Failed to load alive"),
        Ok(_) => (),
    };
    let alive = Arc::new(
        alive_string
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<String>>(),
    );

    let mut dead_string = String::new();
    match OpenOptions::new()
        .read(true)
        .open("dead.txt")
        .unwrap()
        .read_to_string(&mut dead_string)
    {
        Err(_) => eprintln!("Failed to load dead"),
        Ok(_) => (),
    };
    let dead = Arc::new(
        dead_string
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<String>>(),
    );

    let disallowed_ascii = Regex::new(r"[\x{00}-,.-/:-@[-`{-\x{7f}]]").unwrap();
    let words = Arc::new(
        read_file("/usr/share/dict/words")
            .iter()
            .filter(|w| !w.starts_with("#"))
            .filter(|w| !w.trim().is_empty())
            .map(|w| String::from(disallowed_ascii.replace_all(&w.clone(), "")))
            .map(|w| w.to_ascii_lowercase())
            .collect::<Vec<String>>(),
    );

    let tlds_url = String::from("http://data.iana.org/TLD/tlds-alpha-by-domain.txt");
    let skip_domains = ["es", "ng", "ing"];
    let tlds = Arc::new(
        reqwest::get(&tlds_url)
            .unwrap()
            .text()
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("#"))
            .map(|l| l.trim())
            .map(|l| l.to_ascii_lowercase())
            .filter(|l| !skip_domains.contains(&l.as_str()))
            .map(|tld| idna::domain_to_ascii(&tld.as_str()).unwrap())
            .collect::<Vec<String>>(),
    );

    let words_per_thread = words.len() / cpus;
    let mut handles = Vec::with_capacity(cpus);

    for i in 0..cpus {
        let start = i * words_per_thread;
        let end = (i + 1) * words_per_thread - 1;
        let slice = Arc::clone(&words);
        let tlds = Arc::clone(&tlds);
        let dead = Arc::clone(&dead);
        let alive = Arc::clone(&alive);

        let handle = thread::spawn(move || {
            for word in slice[start..end].iter() {
                for tld in tlds.iter() {
                    if tld.len() < 2 || tld.len() >= word.len() {
                        continue;
                    }

                    if word.ends_with(tld) {
                        match try_push_domain(&word, &tld, Arc::clone(&dead), Arc::clone(&alive)) {
                            Err(_) => eprintln!("Failed to push {}", word),
                            Ok(_) => (),
                        };
                    } else if tld.ends_with("s") && word.ends_with(&tld[0..(tld.len() - 1)]) {
                        match try_push_domain(&word, &tld, Arc::clone(&dead), Arc::clone(&alive)) {
                            Err(_) => eprintln!("Failed to push {}", word),
                            Ok(_) => (),
                        };
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn try_push_domain(
    word: &str,
    tld: &str,
    dead: Arc<Vec<String>>,
    alive: Arc<Vec<String>>,
) -> std::io::Result<()> {
    let mut end = word.len() - tld.len();
    if !word.ends_with(tld) {
        end += 1;
    }
    let domain = format!("{}.{}", &word[..end], tld);
    if dead.contains(&domain) || alive.contains(&domain) {
        return Ok(());
    }

    let whois = WhoIs::from_path("servers.json").unwrap();
    let options = WhoIsLookupOptions::from_string(domain.as_str()).unwrap();
    let mut dead_file = OpenOptions::new().append(true).open("dead.txt")?;
    let found = match whois.lookup(options) {
        Err(_) => {
            eprintln!("{} timed out", domain);
            dead_file.write(format!("{}\n", domain).as_bytes())?;
            return Ok(());
        }
        Ok(found) => found
            .lines()
            .filter(|line| line.contains("ERROR"))
            .map(|line| line.to_string())
            .collect::<Vec<String>>(),
    };

    if !found.is_empty() {
        println!("--- {} looks good! ---", domain);
        let mut alive_file = OpenOptions::new().append(true).open("alive.txt")?;
        alive_file.write(format!("{}\n", domain).as_bytes())?;
    }

    Ok(())
}

fn read_file(name: &str) -> Vec<String> {
    let mut strings = String::new();
    let mut file = match OpenOptions::new().read(true).open(name) {
        Err(_) => {
            eprintln!("Failed to open {}", name);
            process::exit(1);
        }
        Ok(file) => file,
    };
    match file.read_to_string(&mut strings) {
        Err(why) => {
            eprintln!("Failed to read {}: {}", name, why);
            process::exit(1);
        }
        Ok(_) => (),
    }
    strings
        .lines()
        .map(|s| String::from(s))
        .collect::<Vec<String>>()
}
