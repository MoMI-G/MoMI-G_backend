extern crate time;

use std::path::Path;
use lib::{Config, OptionalRegion, Database};
use std::process::Command;
use std::process::Stdio;
use std::fs::File;
use std::io::Error;
use std::io::Write;
use iron::Url;
use std::fs::metadata;
use Args;
use regex::Regex;

// Unix-only methods.
use std::os::unix::io::{AsRawFd, FromRawFd};
/*
pub trait Graph <T: Sync+Send> {
    fn nodes_list(self);
    fn generate_graph(self, Region, &Path);
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VG {}

impl<T: Sync + Send> Graph<T> for VG {
    fn nodes_list(self) {
        return ();
    }

    fn generate_graph(self, path: Region, cache_path: &Path) {
    }
}
 */

pub trait Graph {
    fn nodes_list(&self);
    fn generate_graph_to_file(&self, OptionalRegion, i64, &File, &Option<i64>, &Config, &String, bool, &String) -> Result<bool, Error>;
    fn generate_graph_to_file_wo_helper(&self, OptionalRegion, i64, &Path, &Option<i64>, &Config, &String, bool, &String, bool, i32) -> Result<bool, Error>;
    fn version(&self, config: &Config) -> i32;
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VG {}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GraphDB {
   VG(VG),
}
/*
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GraphConfig
    Steps(i64),
    Length(i64)
}

impl GraphConfig {
    fn new(steps: Option<i64>, length: Option<i64>) {

    }
}
*/

impl VG {
    pub fn spawn_vcf_for_visualize(&self, vcf_name: &String, uuid: &String, config: &Config, current_dir: &String, is_pcf: bool, reference: Option<&String>, filename: Option<&String>) -> Result<(), Error>{
        // Spawn and do not wait until end runs.
        // Currently, DUMMY script will running.
        let name = vcf_name.clone();
        let vcf_or_pcf = if is_pcf { "pcf" } else { "vcf" };
        let mut _command1 = Command::new("bash")
            .args(&["script/vcf2xg.sh", &name, &uuid, &config.bin.vg_tmp, &current_dir, vcf_or_pcf, reference.unwrap_or(&("hg19".to_string())), filename.unwrap_or(&("".to_string()))])
            .spawn()?;
        return Ok(())
    }
    fn replace_file_name(region: &OptionalRegion, path: &String) -> String {
        path.replace("{}", &region.path)
    }
    fn download_file(&self, url: &String, filename: &String, _args: &Args) -> Result<(), Error> {
        let mut command3 = Command::new("curl")
            .args(&[url, "-S", "-L", "-k", "-o", &filename, "-m", "100"])
            .spawn()?;

        let _status = command3.wait()?;
        return Ok(());
    }
    pub fn generate_graph_to_file_custom(&self, path: OptionalRegion, data: i64, file: &File, steps: &Option<i64>, config: &Config, json: &Option<String>, xgpath_old: &Option<String>, args: &Args) -> Result<bool, Error> {
        let xgpath_clone = xgpath_old.clone();
        let mut xgpath = xgpath_clone.unwrap_or("".to_string());
        debug!("{:?}", Url::parse(&xgpath));
        if let Ok(url) = Url::parse(&xgpath) {
            xgpath = url.path().last().unwrap_or(&(time::now().to_timespec().sec.to_string() + ".xg").as_ref()).to_string();// unwrap();
            xgpath = format!("{}/xg/{}", args.flag_tmp, xgpath);
            if let Err(_) = metadata(Path::new(&xgpath)) {
                if let Err(_) = self.download_file(&(format!("{}", url)), &xgpath, args) {
                    debug!("Error download: {:?}", url);
                }
            }
        } else {
            if xgpath == "" {
                xgpath = time::now().to_timespec().sec.to_string() + ".json";
            }
            xgpath = format!("{}/xg/{}", args.flag_tmp, xgpath);
        }
        let json_clone = json.clone();
        match &json_clone.unwrap_or("".to_string()).as_ref() {
            &"" => self.generate_graph_to_file_from_vg_to_json(path, data, file, steps, config, &xgpath),
            k => self.generate_graph_to_file_from_json(path, data, file, steps, config, &k.to_string(), &xgpath),
        }
    }


    fn generate_graph_to_file_from_vg_to_json(&self, path: OptionalRegion, _data: i64, file: &File, _steps: &Option<i64>, config: &Config, xgfile: &String) -> Result<bool, Error> {
        let out = unsafe{Stdio::from_raw_fd(file.as_raw_fd())};
        let xgpath = VG::replace_file_name(&path, xgfile); //&config.data[0].source.xg);
        let path = format!("{}", path);
        debug!("{}, {}", xgpath, path);
        let commands: Vec<&str> = config.bin.vg.split(" ").collect();
        let command1 = Command::new("cat")
            .args(&[xgpath])
            .stdout(Stdio::piped())
            .spawn()?;
        //.expect("failed to spawn a process");
        let mut command2 = Command::new(&commands[0])
            .args(&commands[1..])
            .args(&["view", "-j", "-"])
            .stdin(unsafe{Stdio::from_raw_fd(command1.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(out)
            .spawn()?;
            //.expect("failed to spawn a process");
            //.output().unwrap_or_else(|e| {
            //    panic!("failed to execute process: {}", e)
            //});
        let status2 = command2.wait()?;//.expect("failed to wait on child");

        if status2.success() {
            return Ok(true)
        } else {
            return Ok(false)
        }
    }

    pub fn generate_graph_to_file_from_json(&self, _path: OptionalRegion, _data: i64, file: &File, _steps: &Option<i64>, config: &Config, json: &String, xgpath: &String) -> Result<bool, Error> {
        let out = unsafe{Stdio::from_raw_fd(file.as_raw_fd())};
        debug!("Saved: {}", xgpath);
        let mut command2 = Command::new("tee")
            .args(&[xgpath])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut command3 = Command::new("ruby")
            .args(&["proto/graph-helper.rb", &config.bin.vg, xgpath.as_ref()])
            .stdin(unsafe{Stdio::from_raw_fd(command2.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(out)
            .spawn()?;

        let write = command2.stdin.as_mut().unwrap().write_all(json.as_bytes());
        let status2 = command2.wait()?;//.expect("failed to wait on child");
        let status3 = command3.wait()?;//.expect("failed to wait on child");
        return Ok(status2.success() && status3.success() && write.is_ok())
    }

    pub fn test(&self, config: &Config) -> bool {
        let commands: Vec<&str> = config.bin.vg.split(" ").collect();
        let output = Command::new(commands[0])
            .args(&commands[1..])
            .arg("version")
            .output()
            .expect("VG is not exist");
        if output.status.success() {
            info!("VG Version: {}", String::from_utf8_lossy(&output.stdout));
            info!("VG minor version: {}", self.version(config));
        }
        return output.status.success()
    }

    pub fn version(&self, config: &Config) -> i32 {
        let commands: Vec<&str> = config.bin.vg.split(" ").collect();
        let output = Command::new(commands[0])
            .args(&commands[1..])
            .arg("version")
            .output()
            .expect("VG is not exist");
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_re = Regex::new(r"v1\.(\d+)\.").unwrap();
            let caps = version_re.captures(&version).unwrap();
            let cap = caps.get(1).unwrap();
            let i: i32 = cap.as_str().parse().unwrap_or(-1);
            return i;
        }
        let i = -1_i32;
        return i;
    }
}


const MAX_INTERVAL: u64 = 50000;
const MAX_STEP: i64 = 10;

impl Graph for VG {
    fn version(&self, config: &Config) -> i32 {
        self.version(config)
    }
    fn nodes_list(&self) {
        return ();
    }
    fn generate_graph_to_file_wo_helper(&self, path: OptionalRegion, _data: i64, file: &Path, steps: &Option<i64>, config: &Config, xgfile: &String, tmp: bool, max_interval: &String, gam: bool, version: i32) -> Result<bool, Error> {
        let chunk_prefix = format!("{}{}", config.bin.vg_volume_prefix.clone().unwrap_or("".to_string()), file.to_str().unwrap());
        let file = File::create(file)?;
        let out = unsafe{Stdio::from_raw_fd(file.as_raw_fd())};
        let mut steps = steps.unwrap_or(2);
        if steps > MAX_STEP { steps = MAX_STEP;}
        let xgpath = VG::replace_file_name(&path, xgfile); //&config.data[0].source.xg);
        if path.interval() > Some(max_interval.parse::<u64>().unwrap_or(MAX_INTERVAL)) {
            return Ok(false)
        }
        let path_str = format!("{}", path);
        info!("{}, {}", xgpath, path_str);
        // let result = panic::catch_unwind(|| {
        let command: String = match tmp { false => config.bin.vg.clone(), true => config.bin.vg_tmp.clone()};
        let commands: Vec<&str> = command.split(" ").collect();
        info!("{:?}", commands);
        if let Some(ref gam_source) = config.data[0].source.gamindex {
            if gam {
                let steps_str = format!("{}", steps);
                let chunk_command = if version >= 9 {
                    ["chunk", "-t", "4", "-x", xgpath.as_ref(), "-p", path_str.as_ref(), "-c", steps_str.as_ref(), "-g", "-a", gam_source, "-b", chunk_prefix.as_ref(), ""]
                } else {
                    ["chunk", "-t", "4", "-x", xgpath.as_ref(), "-p", path_str.as_ref(), "-c", steps_str.as_ref(), "-g", "-A", "-a", gam_source, "-b", chunk_prefix.as_ref()]
                };
                debug!("{:?}", chunk_command);
            let mut command1 = Command::new(&commands[0])
                .args(&commands[1..])
                .args(&chunk_command)
                .stdout(Stdio::piped())
                .spawn()?;
            //.expect("failed to spawn a process");
            let mut command2 = Command::new(&commands[0])
                .args(&commands[1..])
                .args(&["view", "-j", "-"])
                .stdin(unsafe{Stdio::from_raw_fd(command1.stdout.as_ref().unwrap().as_raw_fd())})
                .stdout(Stdio::piped())
                .spawn()?;
            //.expect("failed to spawn a process");
            let mut command3 = Command::new("ruby")
                .args(&["proto/graph-helper2.rb", &command, xgpath.as_ref(), &path.path, chunk_prefix.as_ref()])
                .stdin(unsafe{Stdio::from_raw_fd(command2.stdout.as_ref().unwrap().as_raw_fd())})
                .stdout(out)
                .spawn()?;
            //.expect("failed to spawn a process");
            let status1 = command1.wait()?;//.expect("failed to wait on child");
            let status2 = command2.wait()?;//.expect("failed to wait on child");
            let status3 = command3.wait()?;//.expect("failed to wait on child");

            if status1.success() && status2.success() && status3.success() {
                return Ok(true)
            } else {
                return Ok(false)
            }
        }}
        let mut command1 = Command::new(&commands[0])
            .args(&commands[1..])
            .args(&["find", "-x", xgpath.as_ref(), "-p", path_str.as_ref(), "-c", format!("{}", steps).as_ref()])
            .stdout(Stdio::piped())
            .spawn()?;
            //.expect("failed to spawn a process");
        let mut command2 = Command::new(&commands[0])
            .args(&commands[1..])
            .args(&["view", "-j", "-"])
            .stdin(unsafe{Stdio::from_raw_fd(command1.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(Stdio::piped())
            .spawn()?;
        //.expect("failed to spawn a process");
        let mut command3 = Command::new("ruby")
            .args(&["proto/graph-helper2.rb", &command, xgpath.as_ref(), &path.path])
            .stdin(unsafe{Stdio::from_raw_fd(command2.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(out)
            .spawn()?;
            //.expect("failed to spawn a process");
        let status1 = command1.wait()?;//.expect("failed to wait on child");
        let status2 = command2.wait()?;//.expect("failed to wait on child");
        let status3 = command3.wait()?;//.expect("failed to wait on child");
        info!("{} {} {}", status1, status2, status3);

        if status1.success() && status2.success() && status3.success() {
            return Ok(true)
        } else {
            return Ok(false)
        }
    }

    fn generate_graph_to_file(&self, path: OptionalRegion, _data: i64, file: &File, steps: &Option<i64>, config: &Config, xgfile: &String, tmp: bool, max_interval: &String) -> Result<bool, Error> {
        let out = unsafe{Stdio::from_raw_fd(file.as_raw_fd())};
        let mut steps = steps.unwrap_or(2);
        if steps > MAX_STEP { steps = MAX_STEP;}
        let xgpath = VG::replace_file_name(&path, xgfile);
        if path.interval() > Some(max_interval.parse::<u64>().unwrap_or(MAX_INTERVAL)) {
            return Ok(false)
        }
        let path = format!("{}", path);
        debug!("{}, {}", xgpath, path);
        let commands: Vec<&str> = match tmp { false => config.bin.vg.split(" ").collect(), true => config.bin.vg_tmp.split(" ").collect() };
        let mut command1 = Command::new(&commands[0])
            .args(&commands[1..])
            .args(&["find", "-x", xgpath.as_ref(), "-p", path.as_ref(), "-c", format!("{}", steps).as_ref()])
            .stdout(Stdio::piped())
            .spawn()?;
            //.expect("failed to spawn a process");
        let mut command2 = Command::new(&commands[0])
            .args(&commands[1..])
            .args(&["view", "-j", "-"])
            .stdin(unsafe{Stdio::from_raw_fd(command1.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(Stdio::piped())
            .spawn()?;
            //.expect("failed to spawn a process");
        let mut command3 = Command::new("ruby")
            .args(&["proto/graph-helper.rb", &config.bin.vg, xgpath.as_ref()])
            .stdin(unsafe{Stdio::from_raw_fd(command2.stdout.as_ref().unwrap().as_raw_fd())})
            .stdout(out)
            .spawn()?;
            //.expect("failed to spawn a process");
            //.output().unwrap_or_else(|e| {
            //    panic!("failed to execute process: {}", e)
        let status1 = command1.wait()?;//.expect("failed to wait on child");
        let status2 = command2.wait()?;//.expect("failed to wait on child");
        let status3 = command3.wait()?;//.expect("failed to wait on child");

        if status1.success() && status2.success() && status3.success() {
            return Ok(true)
        } else {
            return Ok(false)
        }
    }
}

