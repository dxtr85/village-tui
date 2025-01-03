use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Configuration {
    pub asset_dir: PathBuf,
}

impl Configuration {
    pub fn new(dir: &Path) -> Configuration {
        let c_path = dir.join("village-tui.conf");

        if c_path.exists() {
            parse_config(&c_path)
        } else {
            default_config()
        }
    }
}
fn default_config() -> Configuration {
    let mut asset_dir = PathBuf::new();
    asset_dir = asset_dir.join("/home/dxtr/projects/village-tui/assets/");
    Configuration { asset_dir }
}

fn parse_config(file: &Path) -> Configuration {
    let mut asset_dir = PathBuf::new();
    let lines_iter = read_lines(file).unwrap().into_iter();
    for line in lines_iter {
        let ls = line.unwrap().to_string();
        if ls.starts_with('#') || ls.is_empty() {
            eprintln!("Ignoring Line: {}", ls);
        } else {
            eprintln!("Parsing Line: {}", ls);
            let mut split = ls.split_whitespace();
            let line_header = split.next().unwrap();
            match line_header {
                "ASSET_DIR" => {
                    if let Some(dir) = split.next() {
                        asset_dir = PathBuf::new();
                        asset_dir = asset_dir.join(dir);
                    }
                }
                other => {
                    eprintln!("Unrecognized config line: {}", other);
                }
            }
        }
    }
    Configuration { asset_dir }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}
