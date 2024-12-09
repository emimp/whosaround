// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use std::error::Error;
use std::time::Duration;
use tokio::time;
use std::collections::HashMap;
use std::fs::{read_dir, File, OpenOptions};
use std::io::{self, BufRead, Write};
use uuid::Uuid;
use btleplug::api::{BDAddr, Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};

#[derive(Debug)]
struct Device {
    mac_addr: BDAddr,
    local_name: Option<String>,
    tx: Option<i16>,
    manuf: Option<String>,
    rssi: Option<i16>,
    services: Vec<Uuid>,
    services_info: Vec<Option<String>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init()?;

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    println!("{:?}",&adapter_list);

    let mac_vendor_map = load_mac_vendor_map("manuf.txt")?;
    let adapter = adapter_list.first().unwrap().clone();
    
    loop {
        let device_table = check_adapters(adapter.clone(), &mac_vendor_map).await.unwrap();
        let dev = format!("{:#?}",device_table);
        
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true) // Truncate the file so old data is removed
            .open("outp.txt")
            .expect("Unable to open file");
        
        file.write_all(dev.as_bytes())
            .expect("Failed to write to file");
        println!("looped");
        time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn check_adapters(adapter: Adapter, mac_vendor_map: &HashMap<String, String>) -> Result<Vec<Device>, Box<dyn Error>> {
    let mut device_table = Vec::new();
    println!("Starting scan on {}...", adapter.adapter_info().await?);
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");
    time::sleep(Duration::from_secs(1)).await;
    let peripherals = adapter.peripherals().await?;

    for device in &peripherals {
        let mac_addr = device.address();
        
        
        if !device_table.iter().any(|d: &Device| d.mac_addr == mac_addr) {
            let prop = device.properties().await?.unwrap();
            let d = device.is_connected();
            let local_name = prop.local_name;
            let tx = prop.tx_power_level;
            let rssi = prop.rssi;

            let services = prop.services;

            let services_info: Vec<Option<String>> = services.iter().map(|u| {
                let uuid = &u.to_string()[4..8];
                find_service_info("uuids", uuid).unwrap()
            }).collect();

            let manuf = match find_vendor(&mac_vendor_map, &mac_addr) {
                Some(vendor) => Some(vendor.to_string()),
                None => None,
            };

            let device = Device {mac_addr, local_name, tx, manuf, rssi, services, services_info};
            device_table.push(device);
        }

    }
    device_table.sort_by(|a, b| b.rssi.cmp(&a.rssi));
    adapter.stop_scan().await.expect("Couldn't Stop Scan");
    Ok(device_table)
}



/// Reads the file and loads the MAC address mapping.
fn load_mac_vendor_map(file_path: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut mac_vendor_map = HashMap::new();

    // Open the file
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    // Parse each line
    for line in reader.lines() {
        let line = line?;
        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        // Split the line by tabs or spaces
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            // Extract MAC prefix and full vendor name
            let mac_prefix = parts[0].to_string();
            let vendor_name = parts[2..].join(" "); // Join remaining parts for full vendor name
            mac_vendor_map.insert(mac_prefix, vendor_name);
        }
    }

    Ok(mac_vendor_map)
}

/// Checks if the MAC address belongs to a specific vendor.
fn find_vendor<'a>(mac_vendor_map: &'a HashMap<String, String>, mac_address: &'a BDAddr) -> Option<&'a String> {
    let mac_prefix = mac_address.to_string().split(':').take(3).collect::<Vec<&str>>().join(":");
    mac_vendor_map.get(&mac_prefix)
}

fn find_service_info(uuid_dir: &str, uuid: &str) -> Result<Option<String>, Box<dyn Error>> {
    for file in read_dir(uuid_dir)? {
        let file = file?;
        let path = file.path();
        if path.is_file() {
            if let Ok(file) = File::open(&path) {
                let reader = io::BufReader::new(file);
                let mut lines = reader.lines();

                while let Some(Ok(line)) = lines.next() {
                    if line.contains(uuid) {
                        println!("{:?}",line);
                        if let Some(Ok(service_info)) = lines.next() {
                            return Ok(Some(service_info))
                        } else {
                            return Ok(None)
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}
