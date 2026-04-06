// use dotenv::dotenv;
use serde_json::Value;
// use std::env;
use std::fs;
use std::fs::File;

pub fn read_config(file_path: String) -> serde_json::Value {
    // dotenv().ok();
    // Reading the config file path from the environment variable (.env file or system environment variable)
    // let environment = std::env::var("Environment").expect("Environment must be set.");
    // let file_path = "conf/conf.json";

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Error while reading config file: {:?}", e);
            std::process::exit(0);
        }
    };

    // Reading the config file path from the environment variable
    // let args: Vec<String> = env::args().collect();
    // let file = File::open(&args[1]).expect("config file not found!!");
    // let file = File::open("server_config.json").expect("config file not found!!");
    let config: Value = serde_json::from_reader(file).expect("error while reading config file!!");
    // print!("{:?}",config);
    return config;
}

pub fn read_certs(file_path: String) -> std::io::Result<String> {
    // println!("file Path: {:?}",file_path);
    let content = fs::read_to_string(file_path)
        .unwrap()
        .replace("\"", "")
        .replace("\\n", "\n");
    // println!("{:?}", content);
    Ok(content)
}
