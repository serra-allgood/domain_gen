extern crate num_cpus;

use std::fs::File;
use std::io::Read;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let cpus = num_cpus::get();
    let words = Arc::new(read_file("words.txt"));
    let tlds = Arc::new(read_file("tld.txt"));

    let words_per_thread = words.len() / cpus;
    let mut handles = Vec::with_capacity(cpus);
    let domains = Arc::new(Mutex::new(Vec::with_capacity(words.len())));

    for i in 0..cpus {
        let start = i * words_per_thread;
        let end = (i + 1) * words_per_thread - 1;
        let slice = Arc::clone(&words);
        let tlds = Arc::clone(&tlds);
        let domains = Arc::clone(&domains);

        let handle = thread::spawn(move || {
            for word in slice[start..end].iter() {
                for tld in tlds.iter() {
                    if word_too_small(&word, &tld) {
                        continue;
                    }

                    if word_ends_in_tld(&word, &tld) {
                        push_domain(Arc::clone(&domains), &word, &tld, true);
                    } else if possible_plural(&tld)
                        && word_ends_in_tld(&word, &tld[0..(tld.len() - 1)])
                    {
                        push_domain(Arc::clone(&domains), &word, &tld, false);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    domains.lock().unwrap().sort_unstable();
    domains.lock().unwrap().dedup();

    for domain in domains.lock().unwrap().iter() {
        println!("{}", domain);
    }
}

fn possible_plural(tld: &str) -> bool {
    &tld[(tld.len() - 1)..tld.len()] == "s"
}

fn push_domain(domains: Arc<Mutex<Vec<String>>>, word: &str, tld: &str, ends_with: bool) {
    let mut end = word.len() - tld.len();
    if !ends_with {
        end += 1;
    }
    let domain = format!("{}.{}", &word[0..end], tld);

    domains.lock().unwrap().push(domain);
}

fn word_ends_in_tld(word: &str, tld: &str) -> bool {
    let (start, end) = start_end(word, tld);
    &word[start..end] == tld
}

fn word_too_small(word: &str, tld: &str) -> bool {
    word.len() <= tld.len()
}

fn start_end(word: &str, tld: &str) -> (usize, usize) {
    let start = word.len() - tld.len();
    let end = word.len();
    (start, end)
}

fn read_file(name: &str) -> Vec<String> {
    let mut strings = String::new();
    let mut file = match File::open(name) {
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
