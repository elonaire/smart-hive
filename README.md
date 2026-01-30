# Smart Hive (Software-defined Product)🐝
**Hackathon:** #RustAfricaHackathon

This is an IoT/Embedded project that aims to make bee farming more efficient and protect nature while at it. The Smart Hive was born as a result of first-hand experience with the process of traditional beekeeping in Africa; the inefficiency, the lack of precision, the harm done to bees during harvest, and the effect it has on bees rebuilding and making honey once again.

## Project structure
The project is organized into Rust workspaces with dependency crates managed centrally in the workspace. This aims to harmonize the build process. The workspace comprises three crates namely hardware-abstraction (lib), smart-hive (bin) and software-defined-hive (lib).

The reason for this is to separate concerns and start with a software-first approach in developing software-defined IoT products. The other reason is to make it hardware-agnostic so that we can support several microcontroller units (MCUs) in the near future.

## Local Testing
To run this project you need two things:
1. The firmware
2. The hardware (ESP32) or simulator(preferably Wokwi ESP32)

The firmware can be found under build artifacts in releases in the repository.

### Wokwi Setup
`wokwi.toml`
```toml
[wokwi]
version = 1
firmware = 'esp32-mini-1/smart-hive'
elf = 'esp32-mini-1/smart-hive'

[net]
gateway = "private"

# Expose ESP32 services to host
[[net.forward]]
from = "localhost:1883"
to = "target:1883"
```
You only need to copy the binary `smart-hive` to the `esp32-mini-1` directory and run the simulation (preferably in RustRover using the Wokwi plugin).

### MQTT Events
The following are MQTT events which the hive subscribes to:
1. smart-hive/commands
Sample message:
```json
{"command": "authorize_harvest"}
```
Message constraints:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "command")]
pub enum HiveCommand {
    #[serde(rename = "authorize_harvest")]
    AuthorizeHarvest,

    #[serde(rename = "cancel_harvest")]
    CancelHarvest,

    #[serde(rename = "emergency_stop")]
    EmergencyStop,

    #[serde(rename = "reset_fault")]
    ResetFault,

    #[serde(rename = "manual_slide_down")]
    ManualSlideDown,

    #[serde(rename = "manual_slide_up")]
    ManualSlideUp,

    #[serde(rename = "update_policy")]
    UpdatePolicy {
        policy: HarvestPolicyConfigs,
    },

    #[serde(rename = "get_policy")]
    GetPolicy,

    #[serde(rename = "get_status")]
    GetStatus,
}
```
2. smart-hive/sensors/weight
Sample message:
```json
{"weight_g": 4200, "timestamp_s": 1738252800}
```

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments 🙏🏽
- [Rust](https://www.rust-lang.org/)
- [esp-idf-hal](https://github.com/esp-rs/esp-idf-hal)
- [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc)
- [embuild](https://github.com/esp-rs/embuild)
- [Mosquitto](https://mosquitto.org/)
- [log](https://github.com/rust-lang/log)
- [serde](https://github.com/serde-rs/serde)
- [serde_json](https://github.com/serde-rs/json)
- [Wokwi](https://wokwi.com/)
- [RustRover](https://www.jetbrains.com/rust/)

## Inspiration
[Flow Hive](https://www.honeyflow.com/)

My late grandpa (JJ) the best beekeeper I have ever known.

## Authors ✍🏽
- [Elon Aseneka Idiong'o](https://github.com/elonaire)


