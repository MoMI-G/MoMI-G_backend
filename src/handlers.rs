// extern crate serde;
extern crate iron_test;
extern crate params;
extern crate serde_json;
extern crate time;

use iron_send_file::send_file;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{metadata, File};
use std::io::Read;
use std::path::Path;

use annotations::*;
use features::Feature;
use handlers::params::{Params, Value};
use iron::headers::ContentType;
use iron::modifiers::Redirect;
use iron::prelude::*;
use iron::{status, AfterMiddleware, Handler, IronResult, Request, Response};
use lib::{Config, Database, OptionalRegion, Region};
use multipart::server::save::Entries;
use multipart::server::save::SaveResult;
use multipart::server::Multipart;
use router::Router;
use std::collections::BTreeMap;
use std::sync::Arc;
use utils::url_compose;
use vg::{Graph, GraphDB};
use Args;

/// Match a `Result` into its inner value or
/// return `500 Internal Server Error`,
/// or some other provided error using the second variant of this macro.
macro_rules! try_handler {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                return Ok(Response::with((
                    status::InternalServerError,
                    e.description(),
                )))
            }
        }
    };
    ( $e:expr, $error:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with(($error, e.description()))),
        }
    };
}

macro_rules! try_handler_string {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with((status::InternalServerError, e))),
        }
    };
    ( $e:expr, $error:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with(($error, e))),
        }
    };
}

/// Get the value of a parameter in the URI.
/// If the parameter was absent, return `400 Bad Request`.
/// If we could not obtain the parameter list, return `500 Internal Server Error`.
macro_rules! get_http_param {
    ( $r:expr, $e:expr ) => {
        match $r.extensions.get::<Router>() {
            Some(router) => match router.find($e) {
                Some(val) => val,
                None => return Ok(Response::with(status::BadRequest)),
            },
            None => return Ok(Response::with(status::InternalServerError)),
        }
    };
}

/// Get the value of a parameter in the URI.
/// If the parameter was absent, return `400 Bad Request`.
/// If we could not obtain the parameter list, return `500 Internal Server Error`.
macro_rules! get_param_str {
    ( $r:expr, $e:expr ) => {
        match try_handler!($r.get_ref::<Params>()).get($e) {
            Some(&Value::String(ref val)) => val.as_ref(),
            _ => return Ok(Response::with((status::BadRequest))),
        };
    };
}

macro_rules! get_param_optional_str {
    ( $r:expr, $e:expr ) => {
        match try_handler!($r.get_ref::<Params>()).get($e) {
            Some(&Value::String(ref val)) => Some(val.clone()),
            _ => None,
        };
    };
}

macro_rules! get_param_boolean {
    ( $r:expr, $e:expr ) => {
        match try_handler!($r.get_ref::<Params>()).get($e) {
            Some(&Value::String(ref val)) => match val.as_ref() {
                "false" => false,
                _ => true,
            },
            _ => false,
        };
    };
}

pub struct Handlers {
    pub ranged_cache: RangedHandler,
    pub feature: FeatureHandler,
    pub region: RegionHandler,
    pub graph: GraphHandler,
    pub overview: OverViewHandler,
    pub multi_part: MultiPartHandler,
    pub upload: UploadHandler,
}

impl Handlers {
    pub fn new(config: Config, args: Args, database: Database) -> Handlers {
        let config = Arc::new(config);
        let database = Arc::new(database);
        let args = Arc::new(args);
        Handlers {
            ranged_cache: RangedHandler::new(config.clone()),
            feature: FeatureHandler::new(config.clone(), database.clone()),
            region: RegionHandler::new(config.clone(), database.clone()),
            graph: GraphHandler::new(config.clone(), args.clone(), database.clone()),
            overview: OverViewHandler::new(config.clone(), args.clone()),
            multi_part: MultiPartHandler::new(args.clone(), config.clone(), database.clone()),
            upload: UploadHandler::new(config.clone(), args.clone(), database.clone()),
        }
    }
}

pub struct RangedHandler {
    config: Arc<Config>,
}

impl RangedHandler {
    fn new(config: Arc<Config>) -> RangedHandler {
        RangedHandler { config: config }
    }
}

impl Handler for RangedHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref name = get_http_param!(req, "filename");
        let ref features = *self.config.data[0].features;
        let ref item = features.iter().find(|&x| x.name == *name).unwrap();

        let path = Path::new(&item.url);
        let res = Response::new();
        send_file(req, res, path)
    }
}

pub struct OverViewHandler {
    config: Arc<Config>,
    args: Arc<Args>,
}

impl OverViewHandler {
    fn new(config: Arc<Config>, args: Arc<Args>) -> OverViewHandler {
        OverViewHandler {
            config: config,
            args: args,
        }
    }
}

impl Handler for OverViewHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref url_str = &req.url.clone().into();
        let uuid = get_param_optional_str!(req, "uuid");
        let source = get_param_str!(req, "source");
        let tempdir = format!("cache/xg/");

        match &*source {
            "features" => {
                match uuid {
                    Some(ref file) => {
                        let url = try_handler_string!(url_compose(
                            &url_str,
                            &(self.args.flag_api.to_string() + &tempdir + &file + ".pcf")
                        )); //
                        Ok(Response::with((status::Found, Redirect(url))))
                    }
                    None => {
                        match self.config.data[0].source.csv {
                            Some(ref file) => {
                                //TODO() Auto Truncate if the row length exceeds 20,000.
                                let url = try_handler_string!(url_compose(
                                    &url_str,
                                    &(self.args.flag_api.to_string() + &file)
                                ));
                                Ok(Response::with((status::Found, Redirect(url))))
                            }
                            None => Ok(Response::with((status::NoContent))),
                        }
                    }
                }
            }
            "chromosomes" => {
                let ref file = self.config.reference.chroms;
                let url = try_handler_string!(url_compose(
                    &url_str,
                    &(self.args.flag_api.to_string() + &file)
                ));
                Ok(Response::with((status::Found, Redirect(url))))
            }
            "metadata" => {
                let data = &self.config.data[0];
                let json = json!({
                    "name": data.name,
                    "desc": data.desc,
                    "ref_id": data.ref_id,
                    "chr_prefix": data.chr_prefix,
                    "static_files": data.static_files,
                    "alignments": data.source.gamindex.is_some()
                });
                let post = try_handler!(serde_json::to_string(&json), status::BadRequest);
                return Ok(Response::with((status::Ok, post)));
            }
            _ => Ok(Response::with((status::BadRequest))),
        }
    }
}

pub struct RegionHandler {
    config: Arc<Config>,
    database: Arc<Database>,
}

impl RegionHandler {
    fn new(config: Arc<Config>, database: Arc<Database>) -> RegionHandler {
        RegionHandler {
            config: config,
            database: database,
        }
    }
}

impl Handler for RegionHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref format: String =
            get_param_optional_str!(req, "format").unwrap_or("bed".to_string());
        let ref multiple: Option<String> = get_param_optional_str!(req, "multiple");
        let ref bins: Option<u32> =
            get_param_optional_str!(req, "bins").and_then(|t| t.parse::<u32>().ok());
        let ref path: &str = get_param_str!(req, "path");
        if let Some(_) = multiple {
            //let Some(_) = path.to_string().find(",") {
            //let path_vector: Vec<Region> = path.split(",").map(|a| try_handler!(Region::new_with_prefix(a.to_string(), &self.config.data[0].chr_prefix))).collect();
            let path_vector: Vec<Region> = path
                .split(",")
                .flat_map(|a| {
                    Region::new_with_prefix(a.to_string(), &self.config.data[0].chr_prefix)
                })
                .collect();
            let features = regions_to_feature_map(
                &self.config.clone(),
                &format.to_string(),
                path_vector,
                *bins,
            );
            let post = try_handler!(serde_json::to_string(&features), status::BadRequest);
            Ok(Response::with((status::Ok, post)))
        } else {
            let path_struct: Region = try_handler!(Region::new_with_prefix(
                path.to_string(),
                &self.config.data[0].chr_prefix
            ));
            let features = region_to_feature_map(
                &self.config.clone(),
                &format.to_string(),
                path_struct,
                *bins,
            );
            let post = try_handler!(serde_json::to_string(&features), status::BadRequest);
            Ok(Response::with((status::Ok, post)))
        }
    }
}

pub struct FeatureHandler {
    config: Arc<Config>,
    database: Arc<Database>,
}

impl FeatureHandler {
    fn new(config: Arc<Config>, database: Arc<Database>) -> FeatureHandler {
        FeatureHandler {
            config: config,
            database: database,
        }
    }
}

impl Handler for FeatureHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let starts_with: Option<String> = get_param_optional_str!(req, "startsWith");
        let equals: Option<String> = get_param_optional_str!(req, "equals");
        let reference: String =
            get_param_optional_str!(req, "ref").unwrap_or(self.config.data[0].ref_id.clone());
        match starts_with {
            Some(starts) => {
                let mut tmpvec: Vec<String> = Vec::new();
                let mut feature = self
                    .database
                    .gene_name_tree
                    .get(&reference)
                    .unwrap()
                    .range(starts.to_string()..);
                while feature
                    .next()
                    .and_then(|tuple| match tuple.0.starts_with(&starts) {
                        true => {
                            tmpvec.push(tuple.0.to_string());
                            Some(tuple)
                        }
                        false => None,
                    })
                    .is_some()
                {}
                let retval = try_handler!(serde_json::to_string(&tmpvec), status::BadRequest);
                Ok(Response::with((status::Ok, retval)))
            }
            None => match equals {
                Some(equals) => match equals.parse::<u64>() {
                    Ok(number) => match node_id_to_region(self.database.clone(), number) {
                        Some(region) => {
                            let retval =
                                try_handler!(serde_json::to_string(&region), status::BadRequest);
                            Ok(Response::with((status::Ok, retval)))
                        }
                        None => Ok(Response::with((status::NoContent))),
                    },
                    Err(_) => {
                        let mut feature_opt = self
                            .database
                            .gene_name_tree
                            .get(&reference)
                            .unwrap()
                            .get(&equals);
                        match feature_opt {
                            Some(feature) => {
                                let retval = try_handler!(
                                    serde_json::to_string(&(equals, feature)),
                                    status::BadRequest
                                );
                                Ok(Response::with((status::Ok, retval)))
                            }
                            None => Ok(Response::with((status::NoContent))),
                        }
                    }
                },
                None => Ok(Response::with((status::BadRequest))),
            },
        }
    }
}

pub struct MultiPartHandler {
    config: Arc<Config>,
    database: Arc<Database>,
    args: Arc<Args>,
}

impl MultiPartHandler {
    fn new(args: Arc<Args>, config: Arc<Config>, database: Arc<Database>) -> MultiPartHandler {
        MultiPartHandler {
            args: args,
            config: config,
            database: database,
        }
    }

    fn process_entries(&self, entries: Entries) -> IronResult<Response> {
        let tempdir = format!("{}/xg", &self.args.flag_tmp);
        info!("Entries: {:?}", entries);
        /*
        for (name, field) in entries.fields {
            println!("Field {:?}: {:?}", name, field);
        }

        for (name, files) in entries.files {
            println!("Field {:?} has {} files:", name, files.len());
        }
        */

        if let Some(files) = entries.files.get("file") {
            if let Some(file) = files.first() {
                let mut json = BTreeMap::new();
                let path = Path::new(&file.path).file_name().unwrap().to_string_lossy();
                match entries.fields.get("json") {
                    Some(_) => {
                        // when vg's json file
                        let mut file = File::open(&file.path).unwrap();
                        let mut contents = String::new();
                        file.read_to_string(&mut contents).unwrap();
                        return Ok(Response::with((status::Ok, contents)));
                    }
                    None => {
                        // when vcf file
                        json.insert("remote_file", path.clone());
                        let post = try_handler!(serde_json::to_string(&json), status::BadRequest);
                        let _cache_filename = time::now().to_timespec().sec.to_string();
                        let reference = entries.fields.get("ref");
                        // Fork post option.
                        match self.database.graph {
                            GraphDB::VG(ref vg) => {
                                if let Err(_) = vg.spawn_vcf_for_visualize(
                                    &file.path.to_string_lossy().into_owned(),
                                    &path.to_string(),
                                    &self.config,
                                    &tempdir,
                                    Path::new(&file.filename.clone().unwrap()).extension()
                                        == Some(OsStr::new("pcf")),
                                    reference,
                                    file.filename.as_ref(),
                                ) {
                                    debug!("Error on spawning vcf for visulization");
                                }
                            }
                        }
                        return Ok(Response::with((status::Ok, post)));
                    }
                }
            }
        }

        Ok(Response::with((
            status::BadRequest,
            "The request is not including multipart data.",
        )))
    }
}

impl Handler for MultiPartHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let tempdir = format!("{}/xg", &self.args.flag_tmp);
        match Multipart::from_request(req) {
            Ok(mut multipart) => {
                // Fetching all data and processing it.
                // save().temp() reads the request fully, parsing all fields and saving all files
                // in a new temporary directory under the OS temporary directory.
                //let is_json: bool = get_param_boolean!(req, "json");
                //FIXME() save().temp()
                match multipart.save().size_limit(1000000000).with_dir(tempdir) {
                    SaveResult::Full(entries) => self.process_entries(entries),
                    SaveResult::Partial(entries, reason) => {
                        self.process_entries(entries.keep_partial())?;
                        Ok(Response::with((
                            status::BadRequest,
                            format!("error reading request: {}", reason.unwrap_err()),
                        )))
                    }
                    SaveResult::Error(error) => Ok(Response::with((
                        status::BadRequest,
                        format!("error reading request: {}", error),
                    ))),
                }
            }
            Err(_) => Ok(Response::with((
                status::BadRequest,
                "The request is not multipart",
            ))),
        }
    }
}

pub struct UploadHandler {
    config: Arc<Config>,
    database: Arc<Database>,
    args: Arc<Args>,
}

impl UploadHandler {
    fn new(config: Arc<Config>, args: Arc<Args>, database: Arc<Database>) -> UploadHandler {
        UploadHandler {
            config: config,
            database: database,
            args: args,
        }
    }
}

impl Handler for UploadHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref steps = get_param_optional_str!(req, "steps").and_then(|t| t.parse::<i64>().ok());
        let ref _length =
            get_param_optional_str!(req, "length").and_then(|t| t.parse::<i64>().ok());
        let ref json: Option<String> = get_param_optional_str!(req, "json");
        info!("json: {:?}", json);
        let ref xgfile: Option<String> = get_param_optional_str!(req, "xg");
        let ref url_str = &req.url.clone().into();
        info!("{}", url_str);
        let ref path: &str = get_param_str!(req, "path");
        let path_struct: OptionalRegion = try_handler!(OptionalRegion::new_with_prefix(
            path.to_string(),
            &self.config.data[0].chr_prefix
        ));
        info!("{}", path_struct);
        let cache_filename = time::now().to_timespec().sec.to_string() + ".json";
        let cache_str = self.args.flag_tmp.clone() + "/" + &cache_filename;
        let cache_path = Path::new(&cache_str);
        let url = try_handler_string!(url_compose(
            &url_str,
            &(self.args.flag_api.clone() + "cache/" + &cache_filename)
        ));
        info!("{}", url);
        match metadata(cache_path) {
            Ok(ref n) if n.len() > 1 => Ok(Response::with((status::Found, Redirect(url)))),
            _ => {
                let cache_file = try_handler!(File::create(cache_path));
                match self.database.graph {
                    GraphDB::VG(ref vg) => match path_struct.inverted() {
                        Some(true) => Ok(Response::with((status::BadRequest))),
                        _ => {
                            let generate_cache = try_handler!(vg.generate_graph_to_file_custom(
                                path_struct,
                                0,
                                &cache_file,
                                steps,
                                &self.config,
                                json,
                                xgfile,
                                &self.args
                            ));
                            match generate_cache {
                                true => Ok(Response::with((status::Found, Redirect(url)))),
                                false => Ok(Response::with((status::InternalServerError))),
                            }
                        }
                    },
                }
            }
        }
    }
}

pub struct GraphHandler {
    config: Arc<Config>,
    database: Arc<Database>,
    args: Arc<Args>,
}

impl GraphHandler {
    fn new(config: Arc<Config>, args: Arc<Args>, database: Arc<Database>) -> GraphHandler {
        GraphHandler {
            config: config,
            database: database,
            args: args,
        }
    }
}

impl Handler for GraphHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref steps = get_param_optional_str!(req, "steps").and_then(|t| t.parse::<i64>().ok());
        let ref _length =
            get_param_optional_str!(req, "length").and_then(|t| t.parse::<i64>().ok());
        let raw: bool = get_param_boolean!(req, "raw");
        let cache: bool = get_param_boolean!(req, "cache");
        let gam: bool = get_param_boolean!(req, "gam");
        let uuid = get_param_optional_str!(req, "uuid");
        let ref url_str = &req.url.clone().into();
        let ref path: &str = get_param_str!(req, "path");
        let path_struct: OptionalRegion = match uuid {
            Some(_) => try_handler!(OptionalRegion::new(path.to_string())),
            None => try_handler!(OptionalRegion::new_with_prefix(
                path.to_string(),
                &self.config.data[0].chr_prefix
            )),
        };
        info!("{}", path_struct);
        let cache_filename = (if raw { "raw_" } else { "" }).to_string()
            + &uuid.clone().map(|t| t + "_").unwrap_or("".to_string())
            + &path_struct.uuid()
            + ".json";
        let cache_str = self.args.flag_tmp.clone() + "/" + &cache_filename;
        let cache_path = Path::new(&cache_str);
        let url = try_handler_string!(url_compose(
            &url_str,
            &(self.args.flag_api.clone() + "cache/" + &cache_filename)
        ));
        info!("{}, {}", url, cache_str);
        match metadata(cache_path) {
            Ok(ref n) if cache && n.len() > 1 => Ok(Response::with((status::Found, Redirect(url)))),
            _ => {
                let cache_file = try_handler!(File::create(cache_path));
                match self.database.graph {
                    GraphDB::VG(ref vg) => match path_struct.inverted() {
                        Some(true) => Ok(Response::with((status::BadRequest))),
                        _ => match uuid {
                            Some(uuid_exist) => {
                                debug!("uuid: {}", uuid_exist);
                                let generate_cache = match raw {
                                    false => try_handler!(vg.generate_graph_to_file(
                                        path_struct,
                                        0,
                                        &cache_file,
                                        steps,
                                        &self.config,
                                        &format!("/{}/xg/{}.xg", &self.args.flag_tmp, uuid_exist),
                                        true,
                                        &self.args.flag_interval
                                    )),
                                    true => try_handler!(vg.generate_graph_to_file_wo_helper(
                                        path_struct,
                                        0,
                                        &cache_path,
                                        steps,
                                        &self.config,
                                        &format!("/{}/xg/{}.xg", &self.args.flag_tmp, uuid_exist),
                                        true,
                                        &self.args.flag_interval,
                                        gam,
                                        self.database.version
                                    )),
                                };
                                match generate_cache {
                                    true => Ok(Response::with((status::Found, Redirect(url)))),
                                    false => Ok(Response::with((status::InternalServerError))),
                                }
                            }
                            None => {
                                let generate_cache = match raw {
                                    false => try_handler!(vg.generate_graph_to_file(
                                        path_struct,
                                        0,
                                        &cache_file,
                                        steps,
                                        &self.config,
                                        &self.config.data[0].source.xg,
                                        false,
                                        &self.args.flag_interval
                                    )),
                                    true => try_handler!(vg.generate_graph_to_file_wo_helper(
                                        path_struct,
                                        0,
                                        &cache_path,
                                        steps,
                                        &self.config,
                                        &self.config.data[0].source.xg,
                                        false,
                                        &self.args.flag_interval,
                                        gam,
                                        self.database.version
                                    )),
                                };
                                match generate_cache {
                                    true => Ok(Response::with((status::Found, Redirect(url)))),
                                    false => Ok(Response::with((status::InternalServerError))),
                                }
                            }
                        },
                    },
                }
            }
        }
    }
}

pub struct JsonAfterMiddleware;

impl AfterMiddleware for JsonAfterMiddleware {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(ContentType::json());
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use features;
    use iron::Headers;
    use lib;
    use serde_yaml;
    use utils::file_read;
    use vg::VG;

    use self::iron_test::*;

    use super::*;

    #[test]
    fn test_feature_handler() {
        let mut s = String::new();
        let config = "test/config_test.yaml";
        file_read(&config.to_string(), &mut s);
        let deserialized_config: lib::Config = match serde_yaml::from_str(&s) {
            Err(why) => panic!("couldn't parse config:  {:?}", why),
            Ok(conf) => conf,
        };

        let vg_inner = VG {};
        let vg = GraphDB::VG(vg_inner);
        let db_name = "test/db";
        let boolean = true;
        let database = features::tmp_new(vg, &deserialized_config, db_name.to_string(), &boolean);
        let db = Arc::new(database);
        let conf = Arc::new(deserialized_config);
        /*
                let response = request::post("http://localhost:3000/feature?startsWith=DDX",
                                            Headers::new(),
                                            "",
                                            &FeatureHandler::new(conf.clone(), db.clone())).unwrap();
                let result_body = response::extract_body_to_bytes(response);

                assert_eq!(result_body, b"[\"DDX11L1\"]");
        */
        let response2 = request::post(
            "http://localhost:3000/feature?startsWith=ddx",
            Headers::new(),
            "",
            &FeatureHandler::new(conf.clone(), db.clone()),
        )
        .unwrap();
        let result_body2 = response::extract_body_to_bytes(response2);

        assert_eq!(result_body2, b"[]");
    }
}
