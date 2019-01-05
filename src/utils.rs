use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use iron::Url;
use url;

pub fn file_read(path_string: &String, s: &mut String) {
    let path = Path::new(path_string);
    let display = path.display();
    let mut config = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, Error::description(&why)),
        Ok(file) => file,
    };

    //let mut s = String::new();
    match config.read_to_string(s) {
        Err(why) => panic!("couldn't read {}: {}", display, Error::description(&why)),
        Ok(_) => {}
        //print!("{} contains:\n{}", display, s),
    }
}

pub fn url_compose(url: &url::Url, path: &str) -> Result<Url, String> {
    // let url: url::Url = url.into();
    let origin = url.origin().unicode_serialization();
    let new_url = origin + path;
    // println!("{}", new_url);
    Url::parse(&new_url)
}

