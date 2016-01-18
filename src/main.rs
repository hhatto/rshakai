extern crate rshakai;
extern crate getopts;
extern crate hyper;
extern crate url;
extern crate time;
use getopts::Options;
use hyper::Client;
use hyper::status::StatusCode;
use hyper::client::response::Response;
use url::{Url, UrlParser};
use std::{env, thread};
use std::time::Duration;
use std::io::prelude::*;
use std::sync::mpsc::{channel, Sender, Receiver};
use time::now;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use rshakai::{config, indicator};

#[derive(Clone, Copy)]
struct HakaiOption {
    max_concurrency: i32,
    loops: i32,
    verbose: bool,
}

struct WorkerOption {
    opts: HakaiOption,
    conf: config::HakaiConfig,
    indicator_tx: Sender<Option<bool>>,
}

static ALL_MSEC: AtomicUsize = ATOMIC_USIZE_INIT;

fn get_request() -> Client {
    let mut client = Client::new();
    let timeout: Option<Duration> = Some(Duration::new(1, 0));
    client.set_read_timeout(timeout);
    return client;
}

// one request
fn hakai(url: url::Url, options: &HakaiOption, action: &config::Action) -> bool {
    let client = get_request();
    let url = &url.to_string();
    let mut res: Response;

    let t1 = time::now();
    if action.method.to_uppercase() == "POST" {
        res = client.post(url).send().unwrap();
    } else {
        res = client.get(url).send().unwrap();
    }
    let t2 = time::now();
    let diff = (t2 - t1).num_milliseconds() as f32;
    ALL_MSEC.fetch_add(diff as usize, Ordering::SeqCst);

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    if options.verbose {
        println!("Response: url={}, delta={}[msec], body_size={}[byte]",
                 url,
                 diff,
                 body.len());
    }

    if res.status != StatusCode::Ok {
        return false;
    }
    return true;
}

// exec actions
fn hakai_scenario(options: HakaiOption, conf: config::HakaiConfig, tx: Sender<Option<bool>>) {
    let o = &options;
    for action in &conf.actions {
        let host = Url::parse(&conf.domain).unwrap();
        let path = &action.path.to_string();
        let url = UrlParser::new().base_url(&host).parse(path).unwrap();
        tx.send(Some(hakai(url, o, action))).unwrap();
    }
}

fn exec_worker(rx: Receiver<Option<WorkerOption>>) {
    loop {
        match rx.recv().unwrap() {
            Some(wconf) => {
                hakai_scenario(wconf.opts, wconf.conf, wconf.indicator_tx);
            }
            None => {
                break;
            }
        }
    }
}

fn print_usage(opts: Options) {
    print!("{}",
           opts.usage("Usage: rshakai [options] CONFIG_FILE.yaml"));
}

fn main() {
    let mut opt = HakaiOption {
        max_concurrency: 1,
        verbose: false,
        loops: 1,
    };
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt("c", "max-request", "max concurrency request", "C");
    opts.optopt("n", "loop", "scenario exec N-loop", "N");
    opts.optflag("v", "verbose", "verbose log");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(opts);
            return;
        }
    };

    opt.max_concurrency = match matches.opt_str("c") {
        Some(v) => i32::from_str_radix(&v, 10).unwrap(),
        None => 1,
    };
    opt.loops = match matches.opt_str("n") {
        Some(v) => i32::from_str_radix(&v, 10).unwrap(),
        None => 1,
    };
    opt.verbose = matches.opt_present("v");

    let config_file = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(opts);
        return;
    };

    let mut conf = config::HakaiConfig::new();
    conf.load(config_file);

    let (tx, rx) = channel::<Option<bool>>();
    let mut indicator = indicator::Indicator::new(rx, opt.max_concurrency as u32);
    let indicator_handler = thread::spawn(move || indicator.run_forever());

    let mut handles = vec![];
    let mut workers = vec![];

    // gen worker
    for _ in 0..opt.max_concurrency {
        let (worker_tx, worker_rx) = channel::<Option<WorkerOption>>();
        workers.push(worker_tx.clone());
        handles.push(thread::spawn(move || exec_worker(worker_rx)));
    }

    // request for attack
    for cnt in 0..opt.loops {
        let c = conf.clone();
        let cloned_tx = tx.clone();
        let w = WorkerOption {
            opts: opt,
            conf: c,
            indicator_tx: cloned_tx,
        };
        let offset = ((cnt as i32) % opt.max_concurrency) as usize;
        let req = workers[offset].clone();
        req.send(Some(w)).unwrap();
    }

    // exit for worker
    for worker in workers {
        worker.send(None).unwrap();
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // exit for indicator
    tx.send(None).unwrap();
    indicator_handler.join().unwrap();

    // TODO: valid average response time
    let a = ALL_MSEC.fetch_add(0, Ordering::SeqCst);
    println!("all: {}", a as f32);
}
