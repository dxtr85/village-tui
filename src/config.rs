use dapp_lib::prelude::{load_content_from_disk, read_datastore_from_disk, GnomeId, Transport};
use dapp_lib::prelude::{DataType, NetworkSettings};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::logic::Manifest;

pub struct Configuration {
    pub asset_dir: PathBuf,
    pub storage_neighbors: Vec<(GnomeId, NetworkSettings)>,
}

impl Configuration {
    pub async fn new(dir: &Path) -> Configuration {
        //TODO: we should inherit both from main config
        let c_path = dir.join("village-tui.conf");
        let d_path = dir.join("storage");
        // let my_id_str = format!("{}", my_id);
        // eprintln!("My id str: {}", my_id_str);
        let stored_swarms = list_directories_in_pathbuf(d_path);
        let mut storage_neighbors = vec![];
        let mut added_gids = vec![];
        let mut next_v4 = true;
        for sswarm in stored_swarms {
            let sstr = sswarm.to_string_lossy();
            let sstr = sstr.split('-').last().unwrap();
            eprintln!("Parsing str: {}", sstr);
            let g_id: u64 = u64::from_str_radix(&sstr, 16).unwrap();
            let g_id = GnomeId(g_id);
            let dsync_store = sswarm.join("datastore.sync");
            let (zero_type, zero_hash) = if dsync_store.exists() {
                let app_data = read_datastore_from_disk(
                    dsync_store.clone(),
                    // app_data_send.clone(),
                )
                .await;
                if let Ok((dtype, hash)) = app_data.content_root_hash(0) {
                    (dtype, hash)
                } else {
                    (DataType::Data(255), 0)
                }
            } else {
                (DataType::Data(255), 0)
            };
            if zero_hash == 0 {
                continue;
            }
            if let Some(c_zero) = load_content_from_disk(sswarm, 0, zero_type, zero_hash).await {
                let data_vec = vec![c_zero.read_data(0).unwrap()];
                eprintln!("Building Manifest from single data page (for larger manifests this will probably crash)");
                let manifest = Manifest::from(data_vec);
                let mut pubiplen = manifest.pub_ips.len();
                for pub_ip in manifest.pub_ips {
                    if pub_ip.1 == 0 {
                        // This way we will add other IP to our list
                        if pubiplen > 1 {
                            pubiplen = 1;
                        }
                        // We can only communicate with hosts that have control over their
                        // pub port
                        continue;
                    }
                    if pub_ip.0.is_ipv4() {
                        if next_v4 {
                            if !added_gids.contains(&g_id) {
                                eprintln!("Add V4 neighbor {}:{}", pub_ip.0, pub_ip.1);
                                added_gids.push(g_id);
                                storage_neighbors.push((
                                    g_id,
                                    NetworkSettings {
                                        pub_ip: pub_ip.0,
                                        pub_port: pub_ip.1,
                                        nat_type: pub_ip.2,
                                        port_allocation: pub_ip.3,
                                        transport: Transport::UDPoverIP4,
                                    },
                                ));
                                next_v4 = false;
                            }
                        } else if pubiplen == 1 {
                            if !added_gids.contains(&g_id) {
                                eprintln!("Add neighbor {}:{}", pub_ip.0, pub_ip.1);
                                added_gids.push(g_id);
                                storage_neighbors.push((
                                    g_id,
                                    NetworkSettings {
                                        pub_ip: pub_ip.0,
                                        pub_port: pub_ip.1,
                                        nat_type: pub_ip.2,
                                        port_allocation: pub_ip.3,
                                        transport: Transport::UDPoverIP4,
                                    },
                                ));
                            }
                        }
                    } else {
                        if next_v4 && pubiplen == 1 {
                            if !added_gids.contains(&g_id) {
                                eprintln!("Add a neighbor {}:{}", pub_ip.0, pub_ip.1);
                                added_gids.push(g_id);
                                storage_neighbors.push((
                                    g_id,
                                    NetworkSettings {
                                        pub_ip: pub_ip.0,
                                        pub_port: pub_ip.1,
                                        nat_type: pub_ip.2,
                                        port_allocation: pub_ip.3,
                                        transport: Transport::UDPoverIP6,
                                    },
                                ));
                            }
                        } else {
                            if !added_gids.contains(&g_id) {
                                eprintln!("Add V6 neighbor {}:{}", pub_ip.0, pub_ip.1);
                                added_gids.push(g_id);
                                storage_neighbors.push((
                                    g_id,
                                    NetworkSettings {
                                        pub_ip: pub_ip.0,
                                        pub_port: pub_ip.1,
                                        nat_type: pub_ip.2,
                                        port_allocation: pub_ip.3,
                                        transport: Transport::UDPoverIP6,
                                    },
                                ));
                                next_v4 = true;
                            }
                        }
                    }
                }
            } else {
                eprintln!("Failed to load CID-0");
            }
        }

        let mut conf = if c_path.exists() {
            parse_config(&c_path)
        } else {
            default_config()
        };
        conf.storage_neighbors = storage_neighbors;
        conf
    }
}
fn default_config() -> Configuration {
    let mut asset_dir = PathBuf::new();
    asset_dir = asset_dir.join("/home/dxtr/projects/village-tui/assets/");
    Configuration {
        asset_dir,
        storage_neighbors: vec![],
    }
}

fn parse_config(file: &Path) -> Configuration {
    let mut asset_dir = PathBuf::new();
    asset_dir = asset_dir.join("/home/dxtr/projects/village-tui/assets/");
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
    Configuration {
        asset_dir,
        storage_neighbors: vec![],
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

fn list_directories_in_pathbuf(path: PathBuf) -> Vec<PathBuf> {
    let read_result = fs::read_dir(&path);
    if read_result.is_err() {
        eprintln!(
            "Failed to list directories in {:?}:\n{:?}",
            path,
            read_result.err().unwrap()
        );
        return vec![];
    }
    let entries = read_result.unwrap();

    entries
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_dir() {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect()
}
