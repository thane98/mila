use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;

pub fn load_test_file(name: &str) -> Vec<u8> {
    let mut test_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_file_path.push("resources/test/");
    test_file_path.push(name);
    let mut file = OpenOptions::new().read(true).open(test_file_path).unwrap();
    let mut file_contents: Vec<u8> = Vec::new();
    file.read_to_end(&mut file_contents).unwrap();
    file_contents
}
