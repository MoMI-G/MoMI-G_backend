#![feature(rustc_private)]
#![feature(plugin)]
#![allow(dead_code)] // Remove at release.

#[macro_use]
extern crate serde_derive;
extern crate docopt;
#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_json;
extern crate bio;
extern crate libbigwig;
extern crate multipart;
extern crate regex;
extern crate rocks;
extern crate serde_yaml;
extern crate url;

extern crate env_logger;
extern crate iron;
extern crate iron_send_file;
extern crate logger;
extern crate mount;
extern crate router;
extern crate staticfile;
extern crate time;

extern crate flate2;

mod annotations;
mod features;
mod handlers;
mod lib;
mod utils;
mod vg;

use crate::features::tmp_new;
use crate::handlers::*;
use iron::prelude::*;
use iron::Iron;
use logger::Logger;
use router::Router;
use crate::vg::GraphDB;

use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::process;

use docopt::Docopt;
use crate::vg::VG;

const USAGE: &'static str = "
MoMI-G: Modular Multi-scale Integrated Genome Graph Browser Backend.

Usage:
  ggbb [options]
  ggbb (-h | --help)
  ggbb --version

Options:
  -h --help  Show this screen.
  --version  Show version.
  --config=<yaml>  Config file [default: config.yaml].
  --http=<hostport>  Host and Port [default: 127.0.0.1:8081].
  --threads=<thread>  Threads per process [default: 1].
  --tmp=<tmp>  Cache folder [default: ./tmp].
  --static=<static>  Static folder [default: ./static].
  --rocksdb=<rocksdb>  Path for rocksdb storage [default: ./rocksdb].
  --build=<build>  Path for client html [default: ./build].
  --api=<api>  URL for api [default: /api/v1/].
  --interval=<interval>  Max interval on paths [default: 50000].
  -c --cache  Cache a given coordinate list.
  -s --serve  Serve client html.
  -r --reinitrocks  Reinitialize rocks db if true.
  -n --notest  Do not run tests whether vg works.
  -i --onlyinit  Initialize and exit.
  -v --verbose  Force verbose.
";

#[derive(Debug, Deserialize)]
pub struct Args {
    flag_serve: bool,
    flag_onlyinit: bool,
    flag_notest: bool,
    flag_verbose: bool,
    flag_reinitrocks: bool,
    flag_cache: bool,
    flag_config: String,
    flag_http: String,
    flag_tmp: String,
    flag_threads: i32,
    flag_static: String,
    flag_rocksdb: String,
    flag_build: String,
    flag_api: String,
    flag_interval: String,
}

fn main() {
    env_logger::init();
    let (logger_before, logger_after) = Logger::new(None);

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    if args.flag_verbose {
        println!("Run `RUST_LOG=info cargo run` to see logs.");
        println!("{:?}", args);
    }

    let mut s = String::new();
    let http = &args.flag_http.clone();
    utils::file_read(&args.flag_config, &mut s);

    let deserialized_config: lib::Config = match serde_yaml::from_str(&s) {
        Err(why) => panic!("couldn't parse config: {:?}", why),
        Ok(conf) => conf,
    };
    let vg_inner = VG {};

    if !args.flag_notest {
        if !vg_inner.test(&deserialized_config) {
            return;
        }
    }

    let db = &args.flag_rocksdb.clone();
    let vg = GraphDB::VG(vg_inner);
    let database = tmp_new(vg, &deserialized_config, db.clone(), &args.flag_reinitrocks);
    let static_str = &args.flag_static.clone();
    let static_path = Path::new(static_str);
    let cache_str = &args.flag_tmp.clone();
    let cache_path = Path::new(cache_str);
    let build_str = &args.flag_build.clone();
    let build_path = Path::new(build_str);
    let flag_serve = &args.flag_serve.clone();

    if args.flag_onlyinit {
        println!("Initialization completion");
        process::exit(0);
    }

    let api = &args.flag_api.clone();
    let handlers = Handlers::new(deserialized_config, args, database);
    let json_content_middleware = JsonAfterMiddleware;

    let mut router = Router::new();
    router.get("range/:filename", handlers.ranged_cache, "range");
    router.get("feature", handlers.feature, "feature");
    router.get("region", handlers.region, "region");
    router.get("graph", handlers.graph, "graph");
    router.get("overview", handlers.overview, "overview");
    router.post("render", handlers.upload, "fetch");
    router.post("upload", handlers.multi_part, "multi");

    let mut chain = Chain::new(router);
    //chain.link_before(logger_before); // Should be first!
    chain.link_after(json_content_middleware);
    //chain.link_after(logger_after); // Should be last!

    let mut mount = Mount::new();
    mount.mount(&format!("{}static/", &api), Static::new(static_path));
    mount.mount(&format!("{}cache/", &api), Static::new(cache_path));
    mount.mount(api, chain);
    if *flag_serve {
        mount.mount("/", Static::new(build_path));
    }

    let mut chain2 = Chain::new(mount);
    chain2.link_before(logger_before); // Should be first!
    chain2.link_after(logger_after); // Should be last!

    println!("Start server on {}", http);
    Iron::new(chain2).http(http).unwrap();
}
