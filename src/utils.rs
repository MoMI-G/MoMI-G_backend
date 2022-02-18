use iron::Url;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
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
        Ok(_) => {} //print!("{} contains:\n{}", display, s),
    }
}

pub fn url_compose(url: &iron::Url, path: &str) -> Result<Url, String> {
    // let url: url::Url = url.into();
    let urls = url.to_string();
    let url: url::Url = url::Url::parse(&urls).unwrap();
    let origin = url.origin().unicode_serialization();
    let new_url = origin + path;
    // println!("{}", new_url);
    Url::parse(&new_url)
}

/*
pub fn remove_tmp(args: Args) {
    let demo_dir = fs::read_dir(args.flag_tmp);
    delete_dir_contents(demo_dir);
}

fn delete_dir_contents(read_dir_res: Result<ReadDir, io::Error>) {
    if let Ok(dir) = read_dir_res {
        for entry in dir {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_dir() {
                    fs::remove_dir_all(path).expect("Failed to remove a dir");
                } else {
                    fs::remove_file(path).expect("Failed to remove a file");
                }
            };
        }
    };
}
*/
