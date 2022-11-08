use server_discover::config::Config;

#[tokio::main]
async fn main() {
    if !nix::unistd::Uid::effective().is_root() {
        println!("Must be root to run, as Masscan uses its own 'ad hoc TCP/IP stack' that needs direct access to nic/adapters etc");
        std::process::exit(1);
    }
    
    println!("MC Scanner");
    println!(">>>>>>>>>>>>>>>>>>>>");
    dotenv::dotenv().ok();
    
    let config = match Config::parse_args(std::env::args()) {
        Ok(config) => config,
        Err(e) => {
            println!("Error: {}", e); return
        }
    };

    let (scan_config, ip_ranges) = config.get_configs();
    let total_host_ids = ip_ranges.iter().map(|x| { x.num_uniq_ips() }).sum::<u64>();
    println!("Scanning {} total addresses.", total_host_ids);
    
    let go_nogo = server_discover::go_nogo().await;
    match go_nogo {
        Ok(_) => println!("All tests success."),
        Err(e) => { eprintln!("Error nogo: {}", e); return }
    }
    
    println!("---------------------------------------");
    server_discover::run(scan_config, ip_ranges).await;
}