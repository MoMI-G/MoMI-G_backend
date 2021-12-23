use crate::features::Feature;
use crate::lib::{Config, ConfigFeature, Database, Region};
use libbigwig::*;
use rocks::rocksdb::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::mem;
use std::mem::*;
use std::path::Path;
use std::sync::Arc;

pub fn node_id_to_region(database: Arc<Database>, node_id: u64) -> Option<Region> {
    let raw_bytes: [u8; 8] = unsafe { transmute(node_id) };
    let db_name = &database.rocks;
    let db = DB::open(&Options::default(), db_name).unwrap();
    let coord_option = db.get(&ReadOptions::default(), &raw_bytes);
    if let Ok(ref coord_raw) = coord_option {
        let coord_result = Region::new(String::from_utf8(coord_raw.to_vec()).unwrap());
        if let Ok(coord) = coord_result {
            return Some(coord);
        }
    }
    return None;
}

// Ideally, nodes belonging to the same path should be queried only once at a time, and then the feature should be divided for each node.
// Should caching on KVS
pub fn node_id_to_feature(
    config: Arc<Config>,
    database: Arc<Database>,
    node_id: u64,
) -> Vec<Vec<Feature>> {
    let raw_bytes: [u8; 8] = unsafe { transmute(node_id) };
    let db_name = &database.rocks;
    let db = DB::open(&Options::default(), db_name).unwrap();
    let coord_option = db.get(&ReadOptions::default(), &raw_bytes);
    let mut vec: Vec<Vec<Feature>> = vec![];
    if let Ok(ref coord_raw) = coord_option {
        let coord_result = Region::new(String::from_utf8(coord_raw.to_vec()).unwrap());
        if let Ok(mut coord) = coord_result {
            coord.start_minus();
            for feature in config.data[0].features.iter() {
                let path = Path::new(&feature.url);
                let chr_prefix = feature.chr_prefix.clone().unwrap_or("".to_string());
                // println!("Parsing:  {:?}", path);
                match path.extension().unwrap().to_str() {
                    Some("bed") => {
                        vec.push(giggle(feature, &coord, chr_prefix));
                    }
                    Some("bb") => vec.push(libbigbed(feature, &coord, chr_prefix)),
                    Some("bw") => vec.push(libbigwig(feature, &coord, chr_prefix)),
                    _ => debug!("Unsupported format {:?}", path),
                }
            }
        }
    }
    return vec;
}

pub fn regions_to_feature(
    config: &Arc<Config>,
    track_type: &String,
    coord: Vec<Region>,
    bins: Option<u32>,
) -> Vec<Vec<Vec<Feature>>> {
    coord
        .into_iter()
        .map(|a| region_to_feature(config, track_type, a, bins))
        .collect()
}

pub fn regions_to_feature_map(
    config: &Arc<Config>,
    track_type: &String,
    coord: Vec<Region>,
    bins: Option<u32>,
) -> Vec<HashMap<String, Vec<Feature>>> {
    coord
        .into_iter()
        .map(|a| region_to_feature_map(config, track_type, a, bins))
        .collect()
}

pub fn region_to_feature(
    config: &Arc<Config>,
    track_type: &String,
    coord: Region,
    bins: Option<u32>,
) -> Vec<Vec<Feature>> {
    let mut vec: Vec<Vec<Feature>> = vec![];
    for feature in config.data[0].features.iter() {
        let path = Path::new(&feature.url);
        let chr_prefix = feature.chr_prefix.clone().unwrap_or("".to_string());
        match path.extension().unwrap().to_str() {
            Some("bed") if *track_type == "bed".to_string() => {
                vec.push(giggle(feature, &coord, chr_prefix))
            }
            Some("bb") if *track_type == "bed".to_string() => {
                vec.push(libbigbed_simple(feature, &coord, chr_prefix))
            }
            Some("bw") if *track_type == "wig".to_string() => {
                if let Some(bins) = bins {
                    vec.push(libbigwig_stats(feature, &coord, chr_prefix, bins));
                } else {
                    vec.push(libbigwig_simple(feature, &coord, chr_prefix));
                }
            }
            _ => debug!("Unsupported format {:?}", path),
        }
    }
    vec
}

pub fn region_to_feature_map(
    config: &Arc<Config>,
    track_type: &String,
    coord: Region,
    bins: Option<u32>,
) -> HashMap<String, Vec<Feature>> {
    let mut vec: HashMap<String, Vec<Feature>> = HashMap::new();
    for feature in config.data[0].features.iter() {
        let path = Path::new(&feature.url);
        let chr_prefix = feature.chr_prefix.clone().unwrap_or("".to_string());

        match path.extension().unwrap().to_str() {
            Some("bed") if *track_type == "bed".to_string() => {
                vec.insert(feature.url.clone(), giggle(feature, &coord, chr_prefix));
            }
            Some("bb") if *track_type == "bed".to_string() => {
                vec.insert(
                    feature.url.clone(),
                    libbigbed_simple(feature, &coord, chr_prefix),
                );
            }
            Some("bw") if *track_type == "wig".to_string() => {
                if let Some(bins) = bins {
                    vec.insert(
                        feature.url.clone(),
                        libbigwig_stats(feature, &coord, chr_prefix, bins),
                    );
                } else {
                    vec.insert(
                        feature.url.clone(),
                        libbigwig_simple(feature, &coord, chr_prefix),
                    );
                }
            }
            _ => debug!("Unsupported format {:?}", path),
        }
    }
    vec
}

// Ideally, nodes belonging to the same path should be queried only once at a time, and then the feature should be divided for each node.
/*
fn node_id_to_feature_old(
    config: Arc<Config>,
    database: Arc<Database>,
    node_id: u64,
) -> Vec<Feature> {
    let coord_option = database.coordinates.get(&node_id);
    let mut vec: Vec<Feature> = vec![];
    if let Some(coord) = coord_option {
        for feature in config.data[0].features.iter() {
            let path = Path::new(&feature.url);
            let chr_prefix = feature.chr_prefix.clone().unwrap_or("".to_string());
            // println!("Parsing:  {:?}", path);
            match path.extension().unwrap().to_str() {
                Some("bed") => {
                    vec.extend(giggle(feature.url.clone(), coord, chr_prefix));
                }
                Some("") => {
                    vec.extend(giggle(feature.url.clone(), coord, chr_prefix));
                }
                Some("bb") => vec.extend(libbigbed(feature.url.clone(), coord, chr_prefix)),
                Some("bw") => vec.extend(libbigwig(feature.url.clone(), coord, chr_prefix)),
                _ => println!("Unsupported format {:?}", path),
            }
        }
    }

    return vec;
}
*/

fn libbigbed_simple(feature: &ConfigFeature, coord: &Region, prefix: String) -> Vec<Feature> {
    let path = &feature.url;
    let path_loc = CString::new(path.clone()).unwrap();
    let path_str = CString::new(prefix + coord.path.as_ref()).unwrap();

    let mut vec: Vec<Feature> = vec![];
    unsafe {
        if bwInit(1 << 17) != 0 {
            return vec;
        }
        let fp = bbOpen(path_loc.into_raw(), None);
        if fp.is_null() {
            return vec;
        }
        let intervals = bbGetOverlappingEntries(
            fp,
            path_str.into_raw(),
            coord.start as u32,
            coord.stop as u32,
            1,
        );
        if !intervals.is_null() {
            for i in 0..(*intervals).l {
                let start_offset = *(*intervals).start.offset(i as isize) as u64;
                let stop_offset = *(*intervals).end.offset(i as isize) as u64;
                let name = CStr::from_ptr(*(*intervals).str.offset(i as isize))
                    .to_str()
                    .unwrap()
                    .to_owned();
                let splitted_attr = name.split("\t").map(|s| s.to_string()).collect();
                vec.push(Feature {
                    start_offset: start_offset,
                    stop_offset: stop_offset,
                    id: i as u64,
                    name: feature.name.clone(),
                    attributes: splitted_attr,
                    is_reverse: None,
                    value: None,
                });
            }
        }

        if intervals.is_null() {
            bbDestroyOverlappingEntries(intervals);
        }
        bwClose(fp);
        bwCleanup();
    }

    return vec;
}

fn libbigbed(feature: &ConfigFeature, coord: &Region, prefix: String) -> Vec<Feature> {
    let path = &feature.url;
    debug!("{:?} {:?}", path, coord);
    let path_loc = CString::new(path.clone()).unwrap();
    let path_str = CString::new(prefix + coord.path.as_ref()).unwrap();
    let mut vec: Vec<Feature> = vec![];
    unsafe {
        if bwInit(1 << 17) != 0 {
            return vec;
        }
        let fp = bbOpen(path_loc.into_raw(), None);
        if fp.is_null() {
            return vec;
        }
        let intervals = bbGetOverlappingEntries(
            fp,
            path_str.into_raw(),
            coord.start as u32,
            coord.stop as u32,
            1,
        );
        if !intervals.is_null() {
            for i in 0..(*intervals).l {
                let start_offset = if *(*intervals).start.offset(i as isize) <= coord.start as u32 {
                    0
                } else {
                    *(*intervals).start.offset(i as isize) as u64 - coord.start
                };
                let stop_offset = if coord.stop <= *(*intervals).end.offset(i as isize) as u64 {
                    0
                } else {
                    coord.stop - *(*intervals).end.offset(i as isize) as u64
                };
                let name = CStr::from_ptr(*(*intervals).str.offset(i as isize))
                    .to_str()
                    .unwrap()
                    .to_owned();
                let splitted_attr = name.split("\t").map(|s| s.to_string()).collect();
                vec.push(Feature {
                    start_offset: start_offset,
                    stop_offset: stop_offset,
                    id: i as u64,
                    name: feature.name.clone(),
                    attributes: splitted_attr,
                    is_reverse: None,
                    value: None,
                });
            }
        }
        if intervals.is_null() {
            bbDestroyOverlappingEntries(intervals);
        }
        bwClose(fp);
        bwCleanup();
    }
    return vec;
}

#[cfg(test)]
mod tests {
    use super::libbigbed;
    use crate::features::Feature;
    use crate::lib::{ConfigFeature, Region};

    #[test]
    fn it_doesnot_work() {
        let vec: Vec<Feature> = vec![];
        assert_eq!(
            vec,
            libbigbed(
                &ConfigFeature {
                    name: "feature".to_owned(),
                    url: "test/ensGene.bb".to_owned(),
                    chr_prefix: None,
                    viz: None
                },
                &Region {
                    path: "Y".to_owned(),
                    start: 2712790,
                    stop: 2712894,
                },
                "".to_owned(),
            )
        );
    }

    #[test]
    fn it_works() {
        let raw_attr: String =
            "ENST00000387529\t0\t+\t2712894\t2712894\t0\t1\t104,\t0,\tENSG00000210264\tnull"
                .to_owned();
        let attr: Vec<String> = raw_attr.split("\t").map(|s| s.to_string()).collect();
        let feat: Feature = Feature {
            start_offset: 0,
            stop_offset: 0,
            id: 0,
            name: "test/ensGene.bb".to_owned(),
            is_reverse: None,
            value: None,
            attributes: attr,
        };
        let vec: Vec<Feature> = vec![feat];
        assert_eq!(
            vec,
            libbigbed(
                &ConfigFeature {
                    name: "test/ensGene.bb".to_owned(),
                    url: "test/ensGene.bb".to_owned(),
                    chr_prefix: None,
                    viz: None
                },
                &Region {
                    path: "Y".to_owned(),
                    start: 2712790,
                    stop: 2712894,
                },
                "chr".to_owned(),
            )
        );
    }
}

fn libbigwig_stats(
    feature: &ConfigFeature,
    coord: &Region,
    prefix: String,
    bins: u32,
) -> Vec<Feature> {
    let path = &feature.url;
    let path_loc = CString::new(path.clone()).unwrap();
    let read_only = CString::new("r").unwrap();
    let path_str = CString::new(prefix + coord.path.as_ref()).unwrap();
    let mut vec: Vec<Feature> = vec![];
    unsafe {
        if bwInit(1 << 17) != 0 {
            return vec;
        }
        let fp = bwOpen(path_loc.into_raw(), None, read_only.into_raw());
        if fp.is_null() {
            return vec;
        }
        let intervals = bwStats(
            fp,
            path_str.into_raw(),
            coord.start as u32,
            coord.stop as u32,
            bins,
            0 as i32, // Average
        );
        if !intervals.is_null() {
            let len = bins as usize;
            let ptr = intervals as *const f32;
            let slice = std::slice::from_raw_parts(ptr, len);
            for i in 0..bins {
                //let start_offset =
                //    *(*intervals).start.offset(i as isize) as u64;
                //let stop_offset =
                //    *(*intervals).end.offset(i as isize) as u64;
                //let value = *(*intervals).value.offset(i as isize);
                vec.push(Feature {
                    start_offset: coord.start,
                    stop_offset: coord.stop,
                    id: i as u64,
                    name: feature.name.clone(),
                    attributes: vec![],
                    is_reverse: None,
                    value: Some(slice[i as usize]),
                });
            }
        }

        if intervals.is_null() {
            mem::forget(intervals);
        }
        bwClose(fp);
        bwCleanup();
    }
    return vec;
}

fn libbigwig_simple(feature: &ConfigFeature, coord: &Region, prefix: String) -> Vec<Feature> {
    let path = &feature.url;
    let path_loc = CString::new(path.clone()).unwrap();
    let read_only = CString::new("r").unwrap();
    let path_str = CString::new(prefix + coord.path.as_ref()).unwrap();
    let mut vec: Vec<Feature> = vec![];
    unsafe {
        if bwInit(1 << 17) != 0 {
            return vec;
        }
        let fp = bwOpen(path_loc.into_raw(), None, read_only.into_raw());
        if fp.is_null() {
            return vec;
        }
        //let intervals = bwGetValues(
        let intervals = bwGetOverlappingIntervals(
            fp,
            path_str.into_raw(),
            coord.start as u32,
            coord.stop as u32,
        );
        if !intervals.is_null() {
            for i in 0..(*intervals).l {
                let start_offset = *(*intervals).start.offset(i as isize) as u64;
                let stop_offset = *(*intervals).end.offset(i as isize) as u64;
                let value = *(*intervals).value.offset(i as isize);
                vec.push(Feature {
                    start_offset: start_offset,
                    stop_offset: stop_offset,
                    id: i as u64,
                    name: feature.name.clone(),
                    attributes: vec![],
                    is_reverse: None,
                    value: Some(value),
                });
            }
        }

        if intervals.is_null() {
            bwDestroyOverlappingIntervals(intervals);
        }
        bwClose(fp);
        bwCleanup();
    }
    return vec;
}

fn libbigwig(feature: &ConfigFeature, coord: &Region, prefix: String) -> Vec<Feature> {
    let path = &feature.url;
    let path_loc = CString::new(path.clone()).unwrap();
    let read_only = CString::new("r").unwrap();
    let path_str = CString::new(prefix + coord.path.as_ref()).unwrap();
    let mut vec: Vec<Feature> = vec![];
    unsafe {
        if bwInit(1 << 17) != 0 {
            return vec;
        }
        let fp = bwOpen(path_loc.into_raw(), None, read_only.into_raw());
        if fp.is_null() {
            return vec;
        }
        let intervals = bwGetOverlappingIntervals(
            fp,
            path_str.into_raw(),
            coord.start as u32,
            coord.stop as u32,
        );
        if !intervals.is_null() {
            for i in 0..(*intervals).l {
                let start_offset =
                    if *(*intervals).start.offset(i as isize) - coord.start as u32 <= 0 {
                        0
                    } else {
                        *(*intervals).start.offset(i as isize) as u64 - coord.start
                    };
                let stop_offset = if coord.stop as u32 - *(*intervals).end.offset(i as isize) <= 0 {
                    0
                } else {
                    coord.stop - *(*intervals).end.offset(i as isize) as u64
                };
                let value = *(*intervals).value.offset(i as isize);
                vec.push(Feature {
                    start_offset: start_offset,
                    stop_offset: stop_offset,
                    id: i as u64,
                    name: feature.name.clone(),
                    attributes: vec![],
                    is_reverse: None,
                    value: Some(value),
                });
            }
        }

        if intervals.is_null() {
            bwDestroyOverlappingIntervals(intervals);
        }
        bwClose(fp);
        bwCleanup();
    }
    return vec;
}

fn giggle(_feature: &ConfigFeature, _coord: &Region, _prefix: String) -> Vec<Feature> {
    return vec![];
}
