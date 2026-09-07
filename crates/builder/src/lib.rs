use heck::ToSnakeCase;
use std::env;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::fs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn build_config(config_path: String) {
    println!("cargo:rerun-if-changed=config.yaml");

    let out_dir = env::var("OUT_DIR").unwrap();
    let filepath = PathBuf::from(out_dir).join("config.rs");
    let contents = fs::read_to_string(config_path).expect("Failed to read");

    let yaml_struct: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

    yaml_struct["actuators"].as_sequence().unwrap().iter().filter(|actuator| {
        actuator["enabled"].as_bool().unwrap_or(false)
    }).for_each(|actuator| {
        let name = actuator["id"].as_str().unwrap();
        let id = 0;
        let to_idents = |name: &str| {
            (
                format_ident!("{}", name.to_snake_case()),
                format_ident!("{}", name),
            )
        };

        let fields: Vec<TokenStream> = actuator["attributes"].as_mapping().unwrap().iter().map(|(k,v)| {
            let key = k.as_str().unwrap();
            let value = v.as_str().unwrap();
            quote! {
                #key: #value,
            }
        }).collect();

        let (driver_name, driver_type) = to_idents(actuator["hardware"]["driver"].as_str().unwrap());
        let (adapter_name, adapter_type) = to_idents(actuator["modbus"]["adapter"].as_str().unwrap());
        let expanded = quote! {
            let #driver_name = #driver_type {
                #(#fields)*
            };
            let #adapter_name = #adapter_type {

            };
        };
    });
}
