use clap::Parser;
use error_chain::{error_chain, bail};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE, RANGE},
    StatusCode, Url,
};
use std::{path::PathBuf, io::Cursor};
use zip::ZipArchive;

error_chain! {
  foreign_links {
      Io(std::io::Error);
      Reqwest(reqwest::Error);
      Header(reqwest::header::ToStrError);
  }
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    uri: String,
}
struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u32,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: u32) -> Result<Self> {
        if buffer_size == 0 {
            Err("invalid buffer_size, give a value greater than zero.")?;
        }
        Ok(PartialRangeIter {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = HeaderValue;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some(
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
                    .expect("string provided by format!"),
            )
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    match Url::parse(&args.uri) {
        Ok(_) => {
            let url = &args.uri;
            let client = reqwest::blocking::Client::new();
            let response = client.head(url).send()?;

            let test = response.headers();
            let extract = test.get(CONTENT_TYPE).unwrap();
            
            if extract.eq("application/zip") {
                print!("It's a zip file 🕵\n");
                let response = client.get(url).send()?;
                let length = response.content_length().unwrap();
                println!("length is : {}\n", length);
                if length < 101 {
                    bail!("Error in lenght, length is : {}\n", length);
                }
                for range in PartialRangeIter::new(length - 100, length - 1, 100)? {
                    println!("range {:?}", range);
                    let response = client.get(url).header(RANGE, range).send()?;
                    let status = response.status();
                    if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
                        error_chain::bail!("Unexpected server response: {}", status)
                    }
                    let buf = Cursor::new(response.text()?);
                    let zip = ZipArchive::new(buf).expect("Error reading zip file");
                    println!("----------------------------------------------------");
                    let mut list: Vec<&str> = Vec::new();
                    for a in zip.file_names() {
                        print!("In vector format : {}\n", a);
                        list.push(a);
                    }
                    println!("{:?}", list);
                }
            }
        }
        Err(_) => {
            let path: PathBuf = PathBuf::from(args.uri);
            if !path.exists(){
                bail!("No local file encountered")
            }
            let file = std::fs::File::open(path)?;
            let zip = ZipArchive::new(&file).expect("Error reading zip file");
            let mut list: Vec<&str> = Vec::new();
            for a in zip.file_names() {
                print!("In vector format : {}\n", a);
                list.push(a);
            }
            println!("{:?}", list);
        }
    }
    Ok(())
}
