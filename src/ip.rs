use std::net::Ipv4Addr;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Not;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Subnet {
    addr: Ipv4Addr,
    mask: u8,
}

#[derive(Debug)]
pub struct SubnetParseError(());

impl std::str::FromStr for Subnet {
    type Err = SubnetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.split_whitespace().collect::<String>();
        let parts = trimmed.split("/").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(SubnetParseError(()))
        }
        let addr = match parts[0].parse::<Ipv4Addr>() {
            Ok(ok) => ok, Err(_) => return Err(SubnetParseError(()))
        };
        let mask = match parts[1].parse::<u8>() {
            Ok(ok) => ok, Err(_) => return Err(SubnetParseError(()))
        };
        Ok(Subnet {
            addr,
            mask
        })
    }
}

impl Subnet {
    pub fn new(ip: Ipv4Addr, mask: u8) -> Subnet {
        Self { addr: ip, mask: mask }
    }

    pub fn as_string(&self) -> String {
        format!("{}/{}", self.addr.to_string(), self.mask)
    }

    pub fn num_uniq_ips(&self) -> u64 {
        u64::pow(2u64, 32 - self.mask as u32)
    }
    
    #[inline]
    pub fn get_max_uniq_ips() -> u64 {
        4294967296
    }

    fn min_ips(split_subnets: &HashMap<u32, Vec<Subnet>>) -> u32 {
        let mut min = Subnet::get_max_uniq_ips();
        let mut min_thread = 0;
        for (thread_id, subs) in split_subnets {
            // searches for thread with minimised number of ip's
            let total_host_ids = subs.iter().map(|x| { x.num_uniq_ips() }).sum::<u64>();
            if total_host_ids < min { min = total_host_ids; min_thread = *thread_id };
        }
        min_thread
    }
    
    // evenly divide subnets between worker threads
    pub fn calc_thread_subnets(mut subnets: Vec<Subnet>, num_threads: u32) -> HashMap<u32, Vec<Subnet>> {
        // thread number -> subnets
        let mut split_subnets: HashMap<u32, Vec<Subnet>> = HashMap::new();

        subnets.sort_by(|x, y| { y.num_uniq_ips().cmp(&x.num_uniq_ips()) });

        for subnet in subnets {
            // find an empty thread
            let thread_id = (0..num_threads).find(|x| {
                split_subnets.contains_key(x).not()
            }).unwrap_or_else(|| {
                // else assign the least busy thread
                Subnet::min_ips(&split_subnets)
            });
            
            match split_subnets.entry(thread_id) {
                Entry::Occupied(mut ent) => { ent.get_mut().push(subnet); },
                Entry::Vacant(ent) => { ent.insert(vec![subnet]); }
            };
        }
        split_subnets
    }
}