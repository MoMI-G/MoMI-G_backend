extern crate serde_yaml;

use features::FeatureDB;
use regex::Regex;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use vg::GraphDB;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionalRegion {
    pub path: String,
    pub start: Option<u64>,
    pub stop: Option<u64>,
}

impl fmt::Display for OptionalRegion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        match self.start {
            Some(start) => match self.stop {
                Some(stop) => write!(f, "{}:{}-{}", self.path, start, stop),
                None => write!(f, "{}:{}", self.path, start),
            },
            None => write!(f, "{}", self.path),
        }
    }
}

impl OptionalRegion {
    pub fn interval(&self) -> Option<u64> {
        if let Some(start) = self.start {
            if let Some(stop) = self.stop {
                if start < stop {
                    return Some(stop - start);
                } else {
                    return Some(start - stop);
                }
            }
        }
        None
    }

    pub fn inverted(&self) -> Option<bool> {
        if let Some(_start) = self.start {
            if let Some(_stop) = self.stop {
                return Some(self.start > self.stop);
            }
        }
        None
    }

    pub fn new_with_prefix(path: String, chr_prefix: &str) -> Result<Self, Box<Error>> {
        let re = Regex::new(r"^(.+):(\d*)-?(\d*)$").unwrap();
        let caps = try!(re.captures(&path).ok_or("Parse Error"));
        let mut path_str = try!(caps.get(1).ok_or("Parse Path Error")).as_str();

        let path_string: String;
        if chr_prefix.len() == 0 {
            if path_str.starts_with("chr") {
                path_str = &path_str[3..];
            }
            path_string = path_str.to_string();
        } else {
            if path_str.starts_with(chr_prefix) {
                path_str = &path_str[chr_prefix.len()..];
            }
            if path_str.len() < chr_prefix.len() {
                path_string = format!("{}{}", chr_prefix, path_str);
            } else {
                path_string = path_str.to_string()
            }
        }
        let start = caps.get(2).and_then(|t| t.as_str().parse::<u64>().ok());
        let stop = caps.get(3).and_then(|t| t.as_str().parse::<u64>().ok());
        return Ok(OptionalRegion {
            path: path_string,
            start: start,
            stop: stop,
        });
    }

    pub fn new(path: String) -> Result<Self, Box<Error>> {
        let re = Regex::new(r"^(.+):(\d*)-?(\d*)$").unwrap();
        let caps = try!(re.captures(&path).ok_or("Parse Error"));
        let path = try!(caps.get(1).ok_or("Parse Path Error"));
        let start = caps.get(2).and_then(|t| t.as_str().parse::<u64>().ok());
        let stop = caps.get(3).and_then(|t| t.as_str().parse::<u64>().ok());
        return Ok(OptionalRegion {
            path: path.as_str().to_string(),
            start: start,
            stop: stop,
        });
    }

    pub fn uuid(self: &OptionalRegion) -> String {
        return format!("{}", self);
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub path: String, // Requires no prefix
    pub start: u64,
    pub stop: u64,
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{}:{}-{}", self.path, self.start, self.stop)
    }
}

impl Region {
    /*
    pub fn interval(&self) -> u64 {
        if self.inverted() {
            return self.start - self.stop
        } else {
            return self.stop - self.start
        }
    }
    pub fn inverted(&self) -> bool {
        self.start > self.stop
    }*/

    // It is used on converting dna-sequence region to bed-style region.
    pub fn start_minus(&mut self) {
        self.start = self.start - 1;
    }

    pub fn new_with_prefix(path: String, chr_prefix: &str) -> Result<Self, Box<Error>> {
        let re = Regex::new(r"^(.+):(\d+)-?(\d*)$").unwrap();
        let caps = try!(re.captures(&path).ok_or("Parse Error"));
        let mut path_str = try!(caps.get(1).ok_or("Parse Path Error")).as_str();
        let mut path_string: String;
        if chr_prefix.len() == 0 {
            if path_str.starts_with("chr") {
                path_str = &path_str[3..];
            }
            path_string = path_str.to_string();
        } else {
            if path_str.starts_with(chr_prefix) {
                path_str = &path_str[chr_prefix.len()..]; // .replace("chr", "");
            }
            if path_str.len() < chr_prefix.len() {
                path_string = format!("{}{}", chr_prefix, path_str);
            } else {
                path_string = path_str.to_string()
            }
        }
        let start = try!(caps.get(2).ok_or("Parse Start Position Error"));
        let stop = try!(caps.get(3).ok_or("Parse Stop Position Error"));
        let start_str: &str = start.as_str().as_ref();
        let stop_str: &str = stop.as_str().as_ref();
        let start_u64: u64 = try!(start_str
            .parse::<u64>()
            .map_err(|e| "Parse Int Error, ".to_string() + e.description()));
        let stop_u64: u64 = try!(stop_str
            .parse::<u64>()
            .map_err(|e| "Parse Int Error, ".to_string() + e.description()));
        Ok(Region {
            path: path_string,
            start: start_u64,
            stop: stop_u64,
        })
    }

    pub fn new(path: String) -> Result<Self, Box<Error>> {
        let re = Regex::new(r"^(.+):(\d+)-?(\d*)$").unwrap();
        let caps = try!(re.captures(&path).ok_or("Parse Error"));
        let path = try!(caps.get(1).ok_or("Parse Path Error"));
        let start = try!(caps.get(2).ok_or("Parse Start Position Error"));
        let stop = try!(caps.get(3).ok_or("Parse Stop Position Error"));
        let start_str: &str = start.as_str().as_ref();
        let stop_str: &str = stop.as_str().as_ref();
        let start_u64: u64 = try!(start_str
            .parse::<u64>()
            .map_err(|e| "Parse Int Error, ".to_string() + e.description()));
        let stop_u64: u64 = try!(stop_str
            .parse::<u64>()
            .map_err(|e| "Parse Int Error, ".to_string() + e.description()));
        Ok(Region {
            path: path.as_str().to_string(),
            start: start_u64,
            stop: stop_u64,
        })
    }

    pub fn uuid(self: &Region) -> String {
        return format!("{}", self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region_format(path: &str) -> String {
        format!("{}", Region::new(path.to_string()).unwrap())
    }

    #[test]
    fn region_works() {
        assert_eq!(Region::new("".to_string()).ok(), None);
        assert_eq!(Region::new(":10-20".to_string()).ok(), None);
        assert_eq!(
            Region::new("chr1:12000-12001".to_string()).ok(),
            Some(Region {
                path: "chr1".to_string(),
                start: 12000,
                stop: 12001
            })
        );
        assert_eq!(
            Region::new("chr1:1200943-1201000".to_string()).ok(),
            Some(Region {
                path: "chr1".to_string(),
                start: 1200943,
                stop: 1201000
            })
        );
    }

    #[test]
    fn region_format_works() {
        let a = "chr1:12000-12001";
        assert_eq!(region_format(a), a);
        let b = "10:120-120001";
        assert_eq!(region_format(b), b);
    }
}

pub type GeneNameTree = BTreeMap<String, Region>;
pub type GeneNameEachReference = BTreeMap<String, GeneNameTree>;

#[derive(Debug, PartialEq, Serialize)]
pub struct Database {
    pub features: FeatureDB,
    pub rocks: String,
    pub gene_name_tree: GeneNameEachReference,
    pub graph: GraphDB,
    pub version: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigBin {
    pub vg: String,
    pub vg_tmp: String,
    pub vg_volume_prefix: Option<String>,
    pub graphviz: String,
    pub fa22bit: String,
    pub bigbed: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigRef {
    pub chroms: String,
    pub data: Vec<ConfigRefItem>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigRefItem {
    pub name: String,
    pub features: Vec<ConfigFeature>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigSource {
    pub vg: Option<String>,
    pub rocksdb: Option<String>,
    pub xg: String,
    pub gam: Option<String>,
    pub gamindex: Option<String>,
    pub csv: Option<String>,
    // pub json: Option<String>,
    pub reference: Option<String>,
    // pub ref_id: Option<String>,
    pub twobit: Option<String>,
    pub node_index: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigFeature {
    pub name: String,
    pub url: String,
    pub chr_prefix: Option<String>,
    pub viz: Option<String>, // pub ref_id: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigData {
    pub name: String,
    pub desc: Option<String>,
    pub ref_id: String,
    pub source: ConfigSource,
    pub chr_prefix: String,
    pub features: Vec<ConfigFeature>,
    pub static_files: Vec<ConfigFeature>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub bin: ConfigBin,
    pub reference: ConfigRef,
    pub data: Vec<ConfigData>,
}
