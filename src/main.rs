// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use std::error::Error;
use std::time::Duration;
use tokio::time;

use btleplug::api::{BDAddr, Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;

#[derive(Debug)]
struct Device {
    mac_addr: BDAddr,
    local_name: String,
    tx: Option<i16>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init()?;

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    println!("{:?}",adapter_list);
    let mut device_table = Vec::new();

    for adapter in adapter_list.iter() {
        println!("Starting scan on {}...", adapter.adapter_info().await?);
        adapter
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(10)).await;
        let peripherals = adapter.peripherals().await?;
        for device in &peripherals {
            let prop = device.properties().await?.unwrap();
            let local_name = prop.local_name.unwrap_or_default();
            let mac_addr = device.address();
            let tx = prop.tx_power_level;
            if !device_table.iter().any(|d: &Device| d.mac_addr == mac_addr) {
                
                let device = Device {mac_addr, local_name, tx};
                device_table.push(device);
            }
            

        }
        device_table.sort_by(|a, b| b.tx.cmp(&a.tx));

        println!("{:#?}",device_table);

  
    }

    Ok(())
}
