use domain_contract::DeviceId;
use endpoint_runtime::EndpointConfig;
use fake_endpoint_adapter::registry;

fn main() {
    let device_type = std::env::var("DEVICE_TYPE").unwrap_or_else(|_| "fake".into());
    let device_id = std::env::var("DEVICE_ID").unwrap_or_else(|_| "endpoint-1".into());
    let endpoint = registry()
        .create(&EndpointConfig {
            device_type: device_type.clone(),
            device_id: DeviceId(device_id),
        })
        .unwrap_or_else(|error| panic!("endpoint configuration failed: {error}"));
    println!(
        "endpoint runtime ready: type={device_type}, device={:?}",
        endpoint.descriptor().id
    );
    loop {
        std::thread::park();
    }
}
