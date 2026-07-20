#![allow(dead_code)]

#[path = "../error.rs"]
mod error;
#[path = "../whisper.rs"]
mod whisper;

fn main() {
    if let Err(error) = whisper::run_worker(std::env::args().collect::<Vec<_>>().as_slice()) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
