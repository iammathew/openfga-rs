use openfga_common::json::AuthorizationModel;
use std::{env, fs, path::Path};

fn main() {
    let path_string: String = env::args().nth(1).expect("Expected file argument");
    let path = Path::new(&path_string);
    let file = fs::File::open(&path).unwrap();
    let data: AuthorizationModel = serde_json::from_reader(file).unwrap();
    println!("{:?}", data);
}
