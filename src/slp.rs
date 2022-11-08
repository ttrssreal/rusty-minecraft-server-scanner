use serde_json;
use tokio::net::TcpStream;

use slice_of_array::SliceFlatExt;
use tokio::{self, io::{AsyncWriteExt, AsyncReadExt}};
extern crate base64;

#[derive(Debug)]
pub enum Error {
    NoIp,
    InvalidIpType,
    ServerUnreachable,
    CantPingServer(craftping::Error),
    Timeout(tokio::time::error::Elapsed),
    OldSLPFail,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoIp => write!(f, "error: Scan json did not contain ip field"),
            Error::ServerUnreachable => write!(f, "error: Cant connect to server"),
            Error::CantPingServer(e) => write!(f, "error: Cant server list ping server, Craftping error: {}", e),
            Error::InvalidIpType => write!(f, "error: Invalid Ip"),
            Error::Timeout(e) => write!(f, "Timeout occured with slp or tcp: {}", e),
            Error::OldSLPFail => write!(f, "error: Failed to ping with 1.6")
        }
    }
}

impl From<tokio::io::Error> for Error {
    fn from(_: tokio::io::Error) -> Self {
        Error::ServerUnreachable
    }
}

impl From<craftping::Error> for Error {
    fn from(e: craftping::Error) -> Self {
        Error::CantPingServer(e)
    }
}

pub async fn get_server_info(scan_item: &serde_json::Value) -> Result<serde_json::Value, Error> {
    let ip = match scan_item.get("ip") {
        Some(val) => val.as_str(), None => return Err(Error::NoIp)
    };
    let ip_str = match ip {
        Some(val) => val, None => return Err(Error::InvalidIpType)
    };

    let tcp_stream_wait = TcpStream::connect(format!("{}:25565", ip_str));
    let mut tcp_stream = match tokio::time::timeout(std::time::Duration::from_secs(10), tcp_stream_wait).await {
        Ok(val) => val?,
        Err(e) => return Err(Error::Timeout(e))
    };

    let server_info = craftping::tokio::ping(&mut tcp_stream, "localhost", 25565);
    let server_info = match tokio::time::timeout(std::time::Duration::from_secs(10), server_info).await {
        Ok(val) => val?,
        Err(_) => {
            // if not responding to normal slp try https://wiki.vg/Server_List_Ping#1.6 which, "Modern servers recognize this protocol..."
            drop(tcp_stream);
            match tokio::time::timeout(std::time::Duration::from_secs(10), old_slp(ip_str, scan_item)).await {
                Ok(val) => {
                    match val {
                        Ok(val) => return Ok(val),
                        Err(e) => return Err(e)
                    }
                },
                Err(e) =>  return Err(Error::Timeout(e))
            }
        }
    };

    // turn a vec of Players's into something serializable
    let sample = match &server_info.sample {
        Some(players) => Some(players
            .into_iter()
            .map(|x| { (x.name.clone(), x.id.clone()) })
            .collect::<Vec<_>>()),
        None => None
    };

    let fav = if let Some(val) = server_info.favicon {
        base64::encode(val)
    } else {
        String::from("")
    };
    
    let desc = server_info.description;

    Ok(serde_json::json!({
        "ip": ip_str,
        "scan_result": scan_item,
        "version": server_info.version,
        "protocol": server_info.protocol,
        "max_players": server_info.max_players,
        "online_players": server_info.online_players,
        "sample": sample,
        "description": {
            "text": desc.text,
            "bold": desc.bold,
            "italic": desc.italic,
            "underlined": desc.underlined,
            "strikethrough": desc.strikethrough,
            "obfuscated": desc.obfuscated,
            "color": desc.color,
            "extra": format!("{:?}", desc.extra),
        },
        "favicon": fav,
        "mod_info": format!("{:?}", server_info.mod_info),
        "forge_data": format!("{:?}", server_info.forge_data),
    }))
}

pub async fn old_slp(ip_str: &str, scan_item: &serde_json::Value) -> Result<serde_json::Value, Error> {
    let socket = match tokio::net::TcpStream::connect(format!("{}:25565", ip_str)).await {
        Ok(val) => val,
        Err(_) => return Err(Error::OldSLPFail),
    };
    let mut tcp_stream = tokio::io::BufReader::new(socket);
    // server should respond just after these two
    tcp_stream.write_u8(0xfe).await.unwrap();
    tcp_stream.write_u8(0x01).await.unwrap();
    
    tcp_stream.write_u8(0xfa).await.unwrap();
    tcp_stream.write_u16(0x0B).await.unwrap();
    tcp_stream.write_all(&[0x00, 0x4D ,0x00, 0x43, 0x00, 0x7C, 0x00, 0x50, 0x00, 0x69, 0x00, 0x6E, 0x00, 0x67, 0x00, 0x48, 0x00, 0x6F, 0x00, 0x73, 0x00, 0x74]).await.unwrap();
    tcp_stream.write_u8(0x4a).await.unwrap();
    tcp_stream.write_u16(ip_str.len().try_into().unwrap()).await.unwrap();
    let utf16_hostname = ip_str.encode_utf16().map(|x| {x.to_be_bytes()}).collect::<Vec<[u8; 2]>>();
    tcp_stream.write_all(utf16_hostname.flat()).await.unwrap();
    tcp_stream.write_u16(0x63dd).await.unwrap();

    match tcp_stream.read_u8().await {
        Ok(val) => if val != 0xff { return Err(Error::OldSLPFail) }
        Err(_) => { return Err(Error::OldSLPFail) }
    }
    
    let mut server_info: Vec<u8> = Vec::new();
    tcp_stream.read_to_end(&mut server_info).await.unwrap();

    // interpret as u16 string and reverse it
    let title: Vec<u16> = server_info
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_be_bytes([a[0], a[1]]))
        .rev()
        .collect();
    let regular = title.iter().cloned().rev().collect::<Vec<u16>>();
    let full_string = String::from_utf16_lossy(&regular[..]);

    let mut max_players = Vec::new();
    let mut current_players = Vec::new();
    let mut index = 0;

    for char in &title {
        if *char == 0xa7 { index += 1; break; } else { max_players.push(*char); index += 1; }
    }
    for char in &title[index..] {
        if *char == 0xa7 { break; } else { current_players.push(*char); index += 1; }
    }

    let len = title.len();
    let max_players = String::from_utf16_lossy(&max_players.into_iter().rev().collect::<Vec<_>>()[..]);
    let current_players = String::from_utf16_lossy(&current_players.into_iter().rev().collect::<Vec<_>>()[..]);
    let motd = String::from_utf16_lossy(&title.into_iter().rev().collect::<Vec<_>>()[0..len-index-1]);

    Ok(serde_json::json!({
        "ip": ip_str,
        "scan_result": scan_item,
        "full_response": full_string,
        "max_players": max_players,
        "current_player_count": current_players,
        "motd": motd
    }))
}