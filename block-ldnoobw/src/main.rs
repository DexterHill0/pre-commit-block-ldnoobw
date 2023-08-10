use std::{io::prelude::*, ops::Range};
use std::fs::File;

use clap::Parser;
use curl::easy::{Easy2, Handler, WriteError};
use regex::Regex;


const WORD_LIST_NO_LANG: &str = "https://raw.githubusercontent.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words/master";

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    language: String,
    #[arg(long)]
    exclude: Vec<String>,
}

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

struct FoundWord {
    pub file: String,
    pub range: Range<usize>,
    pub word: String,
}

impl std::fmt::Debug for FoundWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "found bad word ({}) in file: {} @ {:?}", self.word, self.file, self.range)
    }
}

fn main() -> Result<(), FoundWord> {
    let args = Cli::parse();

    let word_list = format!("{WORD_LIST_NO_LANG}/{}", args.language);

    let find_all = globmatch::Builder::new("**/**.*").build(env!("CARGO_MANIFEST_DIR")).unwrap();

    let mut exclude = vec![];
    for e in args.exclude.iter() {
        exclude.push(globmatch::Builder::new(e).build_glob().expect("bad exclude glob"));
    }

    let mut easy = Easy2::new(Collector(Vec::new()));
    easy.url(&word_list).unwrap();
    easy.get(true).unwrap();
    easy.perform().unwrap();
    assert_eq!(easy.response_code().unwrap(), 200);

    let contents = easy.get_ref();
    let words = String::from_utf8_lossy(&contents.0);
    let words = words.trim().split('\n').collect::<Vec<_>>().join(r"\b|\b");

    let regex = Regex::new(&format!(r"(\b{}\b)", words)).unwrap();
    let mut buf = String::new();

    'file: for file in find_all.into_iter().flatten() {
        for e in &exclude {
            if e.is_match(&file) {
                continue 'file;
            }
        }

        let f = File::open(&file);
        let mut f = match f {
            Ok(f) => f,
            Err(e) => {
                eprint!("failed to open file: {} ({})", file.file_name().unwrap().to_str().unwrap(), e);
                continue 'file;
            }
        };

        f.read_to_string(&mut buf).unwrap();

        if let Some(c) = regex.captures(&buf) {
            let m = c.get(0).unwrap();
            return Err(FoundWord {
                file: file.as_path().to_str().unwrap().into(),
                range: m.range(),
                word: m.as_str().into(),
            })
        }

        buf.clear();
    }

    Ok(())
}
