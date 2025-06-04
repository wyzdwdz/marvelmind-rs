// Copyright 2025 wyzdwdz <wyzdwdz@gmail.com>
//
// Licensed under the MIT license <LICENSE or https://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to
// those terms.

use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    sync::mpsc::{self, Receiver},
    thread::{sleep, spawn},
    time::{self, SystemTime},
};

use marvelmind::{self as mm, DeviceList};

const LOG_PATH: &str = "E:\\VSRepos\\mm\\log.csv";
const SAVE_ADDRESS: u8 = 11;

fn save_locations(rx: Receiver<DeviceList>, mut outfile: File) {
    let mut update_times = HashMap::<u8, SystemTime>::new();

    loop {
        let Ok(device_list) = rx.recv() else {
            break;
        };

        let devices = device_list.devices();

        for device in devices {
            if !update_times.contains_key(&device.address()) {
                update_times.insert(device.address(), SystemTime::UNIX_EPOCH);
            }

            let prev_time = update_times.get(&device.address()).unwrap();

            if prev_time >= &device.update_time() {
                continue;
            } else {
                update_times.insert(device.address(), device.update_time());
            }

            if device.q() > 0 {
                println!(
                    "address #{:0>3} x {:.3} y {:.3} z {:.3} q {}",
                    device.address(),
                    device.x() as f64 / 1000.0,
                    device.y() as f64 / 1000.0,
                    device.z() as f64 / 1000.0,
                    device.q()
                );
            }

            if device.address() == SAVE_ADDRESS {
                outfile
                    .write(
                        format!(
                            "{};{};{};{};{};{}\n",
                            device.address(),
                            device.x(),
                            device.y(),
                            device.z(),
                            device.q(),
                            device
                                .update_time()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis(),
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }
    }
}

fn main() {
    let version = mm::api_version().unwrap();

    println!("api version: {}", version);

    mm::open_port(30).unwrap();

    println!("open port successfully");

    let mut devices_list = mm::get_device_list().unwrap();

    let mut outfile = File::create(LOG_PATH).unwrap();
    outfile.write("address;x;y;z;q;t\n".as_bytes()).unwrap();

    let (tx, rx) = mpsc::channel();

    spawn(|| save_locations(rx, outfile));

    loop {
        if devices_list.update_last_locations().unwrap() {
            tx.send(devices_list.clone()).unwrap();
        }

        sleep(time::Duration::from_millis(1));
    }
}
