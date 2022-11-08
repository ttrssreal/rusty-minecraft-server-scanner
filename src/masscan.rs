use serde_json;
use tokio::sync::mpsc;
use std::process::Stdio;

use tokio::io::AsyncBufReadExt;


use crate::ip::Subnet;


pub async fn start_mascan(subnet: &Subnet, verifyer: &mpsc::UnboundedSender<serde_json::Value>, port: u16, rate: u64, apply_bl: bool) {
    // FIXME: optimize probably 
    let args = [&subnet.as_string(), &format!("-p{}", port), "-oD", "-", "--rate", &rate.to_string()];
    let mut masscan_cmd = tokio::process::Command::new("masscan")
    .stdout(Stdio::piped())
    // only care about systems found, https://github.com/robertdavidgraham/masscan/blob/master/src/main-status.c ln.206
    .stderr(Stdio::null())
    .args(args)
    .arg(if apply_bl { "--excludefile exclude.conf" } else { "" })
    .spawn()
    .expect("Ensure masscan is installed");
    
    let stdout = match masscan_cmd.stdout.take() {
        Some(val) => val,
        None => { println!("CANT GET STDOUT"); return; }
    };

    let mut output = tokio::io::BufReader::new(stdout).lines();
    
    tokio::spawn(async move {
        let _ = masscan_cmd.wait().await;
    });
    
    while let Some(line) = match output.next_line().await {
        Ok(val) => val,
        Err(_) => return,
    } {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Err(e) = verifyer.send(json) {
                eprintln!("COULDN'T SEND SCAN RESULT!! {}", e);
                return;
            }
        }
    }
}