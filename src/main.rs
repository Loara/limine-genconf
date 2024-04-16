use std::path::{Path, PathBuf};
use blake2::{Digest, Blake2b512};
use clap::Parser;
use std::fs::{read_to_string, read, write};
use std::io::Write;

#[derive(Parser)]
#[command(version, about = "Update checksums in Limine configuration", long_about = None)]
struct Cli {
    #[arg(short, long)]
    config : PathBuf,

    #[arg(short, long)]
    resource : PathBuf,

    #[arg(short, long)]
    esp : PathBuf,
}

fn name_mod(name : &str) -> String {
    name.chars().map(|c| if c.is_alphabetic() {
        c.to_uppercase().collect::<Vec<char>>()
    }
    else if c.is_digit(10) {
        vec![c]
    }
    else {
        vec!['_']
    }).flatten().collect()
}

fn manage_res(res : &std::ffi::OsStr) -> String {
    format!("${{{}_CKS}}", name_mod(res.to_str().unwrap()))
}

fn append_blake(at : &[u8], out : &mut String) {
    for id in at {
        out.push_str(&format!("{:02x}", id));
    }
}

pub struct Translator<'a, I : IntoIterator<Item = &'a str>> where Self : 'a{
    iter : <I as IntoIterator>::IntoIter,
    repl : bool,
    pat : String,
    blk : [u8; 64],
}

impl<'a, I : IntoIterator<Item = &'a str>> Translator<'a, I> where Self : 'a {
    pub fn new(it : I, p : &std::ffi::OsStr, bl : Blake2b512) -> Self {
        Self{
            iter : it.into_iter(),
            repl : false,
            pat : manage_res(p),
            blk : bl.finalize().into(),
        }
    }
}

impl<'a, I : IntoIterator<Item = &'a str>> Iterator for Translator<'a, I> where Self : 'a {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        match self.iter.next() {
            None => return None,
            Some(v) => {
                if self.repl {
                    return Some(v.to_string());
                }
                else if v.starts_with(&self.pat) {
                    self.repl = true;
                    let mut out = format!("{}=", self.pat);
                    append_blake(&self.blk, &mut out);
                    return Some(out);
                }
                else {
                    return Some(v.to_string());
                }
            }
        }
    }
}

fn main() {
    let ch = Cli::parse();

    if !ch.esp.is_dir() {
        panic!("ESP is not a dir");
    }
    else if !ch.resource.is_file() {
        panic!("Resource is not a file");
    }
    else if !ch.config.is_file() {
        panic!("Configuration is not a file");
    }
    let res_from = &ch.resource;
    let res_name = (&ch.resource).file_name().unwrap();
    let res_to = Path::join(&ch.esp, res_name);

    let res_data = read(&ch.resource).unwrap();
    let cfg = read_to_string(&ch.config).unwrap();

    if &res_to != res_from {
        write(&res_to, &res_data).unwrap();
    }

    let mut hash = Blake2b512::new();
    hash.update(&res_data);

    let lns = Translator::new(cfg.lines(), res_name, hash).collect::<Vec<String>>();
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&ch.config).unwrap();

    for l in lns {
        writeln!(&mut out, "{}", l).unwrap();
    }
}
