use bio::io::{bed, gff};
use bio::utils::Strand;
use bio::utils::Strand::*;
use crate::lib::{Config, ConfigFeature};
use crate::lib::{Database, GeneNameEachReference, GeneNameTree, Region};
use rocks::rocksdb::*;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem::*;
use std::path::Path;
use crate::vg::GraphDB;
use crate::vg::GraphDB::VG;

// NodeId to corresponding feature items.
type Features = HashMap<u64, Vec<Feature>>;
pub type FeatureDB = Vec<Features>;

// Move it to graph, needed.
type CoordToNodeId = HashMap<String, Vec<NodeId>>; // Vec<NodeId> required as sorted by coord.

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct NodeId {
    id: u64,
    coord: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub start_offset: u64,
    pub stop_offset: u64,
    pub id: u64,
    pub name: String,
    pub is_reverse: Option<bool>,
    pub attributes: Vec<String>,
    pub value: Option<f32>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FeatureSet {
    feature_set_id: u64,
    dataset_id: Vec<u64>,
    attributes: Option<String>,
}

fn opt_strand_to_opt_bool(strand: Option<Strand>) -> Option<bool> {
    strand.and_then(|strand| match strand {
        Forward => Some(false),
        Reverse => Some(true),
        Unknown => None,
    })
}

fn record_to_nodes(
    record: bed::Record,
    coord_map: &CoordToNodeId,
    bed_id: u64,
    chr_prefix: &Option<String>,
) -> HashMap<u64, Feature> {
    let mut hash_map: HashMap<u64, Feature> = HashMap::new();
    let chr = match *chr_prefix {
        Some(ref k) => record.chrom().replace(k, ""),
        None => record.chrom().to_string(),
    };
    let ref vec = match coord_map.get(&chr) {
        Some(k) => k,
        None => return hash_map,
    };
    let lower_bound_index = match vec.binary_search_by_key(&record.start(), |b| b.coord) {
        Ok(x) => x,
        Err(x) => x,
    };

    hash_map.insert(
        vec[lower_bound_index].id,
        Feature {
            start_offset: vec[lower_bound_index].coord - record.start(),
            stop_offset: 0,
            id: bed_id,
            name: record.name().unwrap_or_default().to_string(),
            is_reverse: opt_strand_to_opt_bool(record.strand()),
            attributes: vec![],
            value: None,
        },
    );

    let mut index = lower_bound_index;
    while vec.len() > index + 1 && vec[index + 1].coord < record.end() {
        index += 1;
        hash_map.insert(
            vec[index].id,
            Feature {
                start_offset: 0,
                stop_offset: 0,
                id: bed_id,
                name: record.name().unwrap_or_default().to_string(),
                is_reverse: opt_strand_to_opt_bool(record.strand()),
                attributes: vec![],
                value: None,
            },
        );
    }
    return hash_map;
}

// tmpNew should be replecated with a novel implementation.
// Required input list is sorted by coordinates.
//pub fn tmp_new(graph: Arc<Graph>, config: &Config) -> Database {
pub fn tmp_new(graph: GraphDB, config: &Config, db_name: String, rocksdb_init: &bool) -> Database {
    let chroms = vec![
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        "17", "18", "19", "20", "21", "22", "X", "Y",
    ];
    let hashmap = CoordToNodeId::new();
    if *rocksdb_init || !Path::new(&db_name).exists() {
        if let Ok(cf) = DB::open(
            &Options::default().map_db_options(|db| db.create_if_missing(true)),
            db_name.clone(),
        ) {
            'iter: for chr in chroms.iter() {
                if let Some(ref path) = config.data[0].source.node_index {
                    let ref prefix = config.data[0].chr_prefix;
                    let chr_name = prefix.clone() + chr;
                    let path_string = path.clone().replace("{}", &chr_name);
                    let path = Path::new(&path_string);
                    debug!("Chromosome:  {:?}, {:?}", chr, path);

                    let file = match File::open(path) {
                        Ok(f) => f,
                        Err(e) => {
                            debug!("could not open {}; skipping.", e.description());
                            continue 'iter;
                        }
                    };
                    /*
                    let file_gz = match extract_file(path) {
                        Ok(f) => f,
                        Err(e) => {continue 'iter;}
                    };
                    */

                    let br = BufReader::new(file);
                    let mut last_node: Option<NodeId> = None;
                    for line in br.lines() {
                        match line {
                            Ok(l) => {
                                let items: Vec<u64> =
                                    l.split("\t").map(|a| a.parse::<u64>().unwrap()).collect();
                                if items.len() > 1 {
                                    if let Some(item) = last_node {
                                        let reg = Region {
                                            path: (*chr).to_string(),
                                            start: item.coord,
                                            stop: items[1],
                                        };
                                        let raw_bytes: [u8; 8] = unsafe { transmute(item.id) };
                                        if let Err(err) = cf.put(
                                            &WriteOptions::default(),
                                            &raw_bytes,
                                            reg.uuid().as_bytes(),
                                        ) {
                                            debug!("{:?} at {}", err, item.id)
                                        }
                                    }
                                    last_node = Some(NodeId {
                                        id: items[0],
                                        coord: items[1],
                                    });
                                } else {
                                    continue;
                                }
                            }
                            Err(e) => {
                                debug!("ignoring error {}", e);
                                continue;
                            }
                        };
                    }
                    if let Some(item) = last_node {
                        // coord.insert(item.id, Region{ path: (*chr).to_string(), start: item.coord, stop: item.coord + 1000 }); //Todo seems to wrong code.
                        let reg = Region {
                            path: (*chr).to_string(),
                            start: item.coord,
                            stop: item.coord + 1000,
                        };
                        let raw_bytes: [u8; 8] = unsafe { transmute(item.id) };
                        if let Err(err) =
                            cf.put(&WriteOptions::default(), &raw_bytes, reg.uuid().as_bytes())
                        {
                            debug!("{:?} at {}", err, item.id)
                        }
                    }
                }
            }
        }
    }
    let mut vec: FeatureDB = FeatureDB::new();
    let mut gene_per_ref = GeneNameEachReference::new();
    for data in config.reference.data.iter() {
        let mut gene: GeneNameTree = GeneNameTree::new();
        for feature in data.features.iter() {
            // It limits only "config,reference Items."
            let path = Path::new(&feature.url);
            info!("Parsing:  {:?}", path);
            match path.extension().unwrap_or_default().to_str() {
                Some("bed") => {
                    vec.push(tmp_new_internal(feature, &graph, &hashmap));
                }
                Some("gff3") => {
                    tmp_new_gene_internal(feature, &mut gene, gff::GffType::GFF3);
                }
                Some("gtf") => {
                    tmp_new_gene_internal(feature, &mut gene, gff::GffType::GTF2);
                }
                _ => println!("Unsupported format {:?}", path),
            }
        }
        gene_per_ref.insert(data.name.clone(), gene);
    }
    match graph {
        VG(graph2) => {
            let version = graph2.version(config);
            println!("{}", version);
            return Database {
                features: vec,
                //coordinates: coord,
                rocks: db_name,
                gene_name_tree: gene_per_ref,
                graph: VG(graph2),
                version: version,
            };
        }
    };
}

// It includes only "gene" row.
fn tmp_new_gene_internal(feature: &ConfigFeature, gene: &mut GeneNameTree, gff_type: gff::GffType) {
    let gff3 = &feature.url;
    let path = Path::new(&gff3);
    let mut reader = match gff::Reader::from_file(path, gff_type) {
        Ok(f) => f,
        Err(e) => {
            debug!("could not open {}; skipping.", e.description());
            //return result?;
            return;
        }
    };

    let mut index = 0;
    for record in reader.records() {
        index += 1;
        match record {
            Ok(rec) => match rec.feature_type() {
                "gene" => {
                    let reg = match opt_strand_to_opt_bool(rec.strand()) {
                        Some(false) => Region {
                            path: rec.seqname().to_string(),
                            stop: *rec.start(),
                            start: *rec.end(),
                        },
                        _ => Region {
                            path: rec.seqname().to_string(),
                            start: *rec.start(),
                            stop: *rec.end(),
                        },
                    };
                    match rec.attributes().get("gene_name") {
                        Some(name) => gene.insert(name.clone().to_string(), reg),
                        None => continue,
                    };
                }
                _ => continue,
            },
            Err(_) => continue,
        }
    }
    debug!("{} lines processed. end.", index);
}

fn tmp_new_internal(
    feature: &ConfigFeature,
    _graph: &GraphDB,
    hashmap: &CoordToNodeId,
) -> Features {
    let bed = &feature.url;
    let path = Path::new(&bed);
    let mut features: Features = Features::new();

    let mut reader = match bed::Reader::from_file(path) {
        Ok(f) => f,
        Err(e) => {
            debug!("could not open {}; skipping.", e.description());
            return features;
        }
    };
    let mut index: u64 = 0;

    for record in reader.records() {
        let rec = record.ok().expect("Error reading record.");
        let nodes = record_to_nodes(rec, &hashmap, index, &feature.chr_prefix);
        for (key, value) in nodes.into_iter() {
            features.entry(key).or_insert(Vec::new()).push(value);
        }
        index += 1;
    }
    return features;
}
/*
fn extract_file(path_compressed: &Path) -> io::Result<Vec<u8>>{
    let mut v = Vec::new();
    let f = try!(File::open(path_compressed));
    try!(try!(GzDecoder::new(f)).read_to_end(&mut v));
    Ok(v)
}

fn decode_reader(string: &String) -> io::Result<String> {
    let mut gz = GzDecoder::new(string.as_bytes())?;
    let mut s = String::new();
    gz.read_to_string(&mut s)?;
    Ok(s)
}
*/
