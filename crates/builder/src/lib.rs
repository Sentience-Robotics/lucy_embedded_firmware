use heck::ToSnakeCase;
use std::env;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::fs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use lucy_embedded_firmware_core::drivers::pwm_servo;

pub fn build_config(config_path: String) {
    println!("cargo:warning=BUIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIILDING");
    println!("cargo:rerun-if-changed=config.yaml");

    let out_dir = env::var("OUT_DIR").unwrap();
    let destpath = PathBuf::from(out_dir).join("config.rs");
    let contents = fs::read_to_string(config_path).expect("Failed to read");

    let yaml_struct: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

    yaml_struct["actuators"].as_sequence().unwrap().iter().filter(|actuator| {
        actuator["enabled"].as_bool().unwrap_or(false)
    }).for_each(|actuator| {
        let actuator_ident = format_ident!("{}", actuator["id"].as_str().unwrap());
        let actuator_config_class = format_ident!("{}Config", actuator["type"].as_str().unwrap());
        let actuator_driver_class = format_ident!("{}Driver", actuator["type"].as_str().unwrap());
        let actuator_modbus_class = format_ident!("{}Driver", actuator["type"].as_str().unwrap());
        let to_idents = |name: &str| {
            (
                format_ident!("{}", name.to_snake_case()),
                format_ident!("{}", name),
            )
        };

        let config: Vec<TokenStream> = actuator["config"].as_mapping().unwrap().iter().map(|(k,v)| {
            let key = format_ident!("{}", k.as_str().unwrap());
            let value = v.as_u64().unwrap();
            quote! {
                #key: #value,
            }
        }).collect();

        //let (driver_name, driver_type) = to_idents(actuator["driver"].as_str().unwrap());
        let (adapter_name, adapter_type) = to_idents(actuator["modbus"]["adapter"].as_str().unwrap());
        let expanded = quote! {
            pub struct ModbusVariables {
                #actuator_ident: #adapter_type,
            }

            impl ModbusVariables {
                pub fn tick(&self mut) {

                }
            }

            pub fn pre_config() {
                let tmp = &#actuator_config_class {
                    #(#config)*
                };
            }
        };
        println!("cargo:warning={}", expanded);
        std::fs::write(&destpath, expanded.to_string()).expect("Failed to write file");
    });
    let expanded = quote! {
        pub struct ModbusAdapters {

        }

        impl ModbusAdapters {
            pub fn tick(&mut self) {

            }
        }

        pub fn pre_config() {

        }

        pub fn post_config() {

        }
    };
}
