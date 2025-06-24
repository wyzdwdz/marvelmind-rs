# marvelmind-rs

Marvelmind<sup>&copy;</sup> api wrapper in Rust

## Example

```rust
use marvelmind as mm;

let version = mm::api_version().unwrap();
println!("api version: {}", version);

mm::open_port(30).unwrap();
println!("open port successfully");

let mut devices_list = mm::get_device_list().unwrap();
let _ = devices_list.update_last_locations().unwrap();

let devices = device_list.devices();
for device in devices {
    println!(
        "address #{:0>3} x {:.3} y {:.3} z {:.3} q {}",
        device.address(),
        device.x() as f64 / 1000.0,
        device.y() as f64 / 1000.0,
        device.z() as f64 / 1000.0,
        device.q()
    );
}
```

