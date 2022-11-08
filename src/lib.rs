use std::sync::Arc;

use ip::Subnet;
use tokio::sync::mpsc;

pub mod ip;
pub mod config;
pub mod masscan;
pub mod database;
pub mod slp;

pub async fn run(config: config::ScannerConfig, ip_ranges: Vec<Subnet>) {
    let num_threads = config.num_threads;
    let subnets = Arc::new(Subnet::calc_thread_subnets(ip_ranges, num_threads));
    let mut threads = Vec::new();
    
    let (sender, receiver) = mpsc::unbounded_channel::<serde_json::Value>();
    
    println!("Starting server verifyer.");
    let verifyer = tokio::spawn(make_verifyer(receiver));

    println!("Starting threads:");
    for i in 0..num_threads {
        let thread_subnets = Arc::clone(&subnets);
        let thread_send_verifyer = mpsc::UnboundedSender::clone(&sender);
        threads.push(tokio::spawn(async move {
            println!("- Started Thread({})", i);
            let ip_ranges = match thread_subnets.get(&i) {
                Some(some) => some, None => {
                    println!("Thread ({}) has no ip ranges supplied.", i);
                    return;
                }
            };
            for subnet in ip_ranges {
                println!("Sarted Thread ({}) Masscan on {}", i, subnet.as_string());
                masscan::start_mascan(subnet, &thread_send_verifyer, config.port.unwrap(), config.rate, config.apply_blacklist).await;
                println!("Finished Thread ({}) scanning {}", i, subnet.as_string());
            }
        }));
    }
    
    for thread in threads {
        thread.await.unwrap();
    }
    println!("Finished scanning.");
    sender.send(serde_json::json!({"stop":true})).unwrap();
    let _ = verifyer.await;
}

pub async fn make_verifyer(mut rx: mpsc::UnboundedReceiver<serde_json::Value>) {
    let session = Arc::new(database::DbSession::new("mc-server-db").await); // hard coded
    let mut verifyers = Vec::new();
    
    while let Some(target) = rx.recv().await {
        if let Some(_) = target.get("stop") {
            futures::future::join_all(verifyers).await;
            println!("Stoping server verifyer.");
            return;
        }
        let session = Arc::clone(&session);
        verifyers.push(tokio::spawn(async move {
            let server_info = match slp::get_server_info(&target).await {
                Ok(val) => val, Err(e) => {
                    eprintln!("Failed, {}, {}", e, target["ip"]);
                    return;
                }
            };
            if let Err(e) = session.add_server_json(&server_info).await {
                println!("Couldn't insert server info! error: {}", e);
            } else {
                println!("Found server: {:?}", target["ip"]);
            };
        }));
    }
}

pub async fn go_nogo() -> Result<(), Box<dyn std::error::Error>> {
    let session = database::DbSession::new("mc-server-db").await;
    session.client.database("mc-server-db").run_command(mongodb::bson::doc! {"ping": 1}, None).await?; // hard coded
    println!("Go for database");
    Ok(())
}