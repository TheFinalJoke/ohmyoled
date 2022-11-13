use std::{env, path::PathBuf};
fn main() {
    let mut protos = Vec::new();
    protos.push("./proto/weather.proto"); 
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    for proto_file in protos {
        println!("Building");
        tonic_build::configure()
        .build_server(false)
        .out_dir("./src")
        .compile(&[proto_file], &["."])
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));
  
        println!("cargo:rerun-if-changed={}", proto_file);
    }    
}

