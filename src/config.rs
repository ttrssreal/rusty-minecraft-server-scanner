use crate::ip::Subnet;
use std::env;
use serde_yaml;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ScannerConfig {
    pub num_threads: u32,
    pub rate: u64,
    pub apply_blacklist: bool,
    pub default_port: bool,
    pub port: Option<u16>,
}

#[derive(Debug)]
pub struct Config {
    scanner_conf: ScannerConfig,
    ip_ranges: Vec<Subnet>,
}

#[derive(Debug)]
pub enum ArgError {
    NoConfig,
    NoIpRanges,
    PortNotSpecified,
    ConfigParseError(serde_yaml::Error),
    CantOpenConfig(std::io::Error, String)
}

impl std::error::Error for ArgError {}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgError::NoConfig => write!(f, "No config yaml file."),
            ArgError::NoIpRanges => write!(f, "No ipranges file."),
            ArgError::PortNotSpecified => write!(f, "A non-default port needs to be specified."),
            ArgError::ConfigParseError(err) => write!(f, "Can't parse config yaml: {}", err),
            ArgError::CantOpenConfig(err, filename) => write!(f, "Can't open file (\"{}\"): {}", filename, err)
        }
    }
}

impl From<serde_yaml::Error> for ArgError {
    fn from(err: serde_yaml::Error) -> Self { ArgError::ConfigParseError(err) }
}

impl Config {
    pub fn parse_args(mut args: env::Args) -> Result<Config, ArgError> {
        args.next();
        let config_file_loc = match args.next() {
            Some(arg) => arg,
            None => return Err(ArgError::NoConfig)
        };
        let config_file = match std::fs::File::open(&config_file_loc) {
            Ok(file) => file,
            Err(e) => return Err(ArgError::CantOpenConfig(e, config_file_loc))
        };
        let mut scanner_conf: ScannerConfig = serde_yaml::from_reader(config_file)?;
        scanner_conf.port = 
        if scanner_conf.default_port {
            Some(25565)
        } else {
            Some(match scanner_conf.port {
                Some(val) => val,
                None => return Err(ArgError::PortNotSpecified)
            })
        };

        let ip_ranges_file_loc = match args.next() {
            Some(arg) => arg,
            None => return Err(ArgError::NoIpRanges)
        };
        let ip_ranges = 
            match std::fs::read_to_string(&ip_ranges_file_loc) {
                Ok(file) => file,
                Err(e) => return Err(ArgError::CantOpenConfig(e, ip_ranges_file_loc))
            }
            .lines()
            .filter_map(|x| { x.parse::<Subnet>().ok() })
            .collect::<Vec<Subnet>>();
        Ok(Self { scanner_conf, ip_ranges })
    }

    pub fn get_configs(self) -> (ScannerConfig, Vec<Subnet>) {
        (self.scanner_conf, self.ip_ranges)
    }
}