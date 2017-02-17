extern crate byteorder;

mod mobiless;

#[cfg(not(target_os = "emscripten"))]
use std::fs::File;

#[cfg(not(target_os = "emscripten"))]
use std::io::{Read, Write};

#[cfg(not(target_os = "emscripten"))]
fn help() {
    println!("Usage: mobiless source.mobi output.mobi");
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        3 => {
            let mut src_file = File::open(args[1].to_owned()).unwrap();
    
            // load the file
            let mut data = vec![];
            let length = src_file.read_to_end(&mut data).unwrap();
            println!("File loaded with length: {}", length);

            let mut file = mobiless::MobiFile::new(&mut data, length);

            println!("Removing sources...");
            file.remove_sources();
            let mut output = File::create(args[2].to_owned()).unwrap();
            output.write(&file.data[0..file.length]).unwrap();
        },
        _ => {
            help();
            std::process::exit(1);
        },
    }
}