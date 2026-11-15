//! Synthetic, redistributable raw-graph memory comparison; see docs/packed-graph.md.

use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Write};
use std::hash::Hasher;
use std::io::{self, Read};

use step_io::parser::{parse, parse_compact};

const HEADER: &str = "ISO-10303-21;HEADER;FILE_DESCRIPTION(('synthetic points'),'2;1');\
FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;";

struct Points {
    next: u64,
    count: u64,
    bytes: io::Cursor<Vec<u8>>,
}

impl Points {
    fn new(count: u64) -> Self {
        Self {
            next: 0,
            count,
            bytes: io::Cursor::new(HEADER.as_bytes().to_vec()),
        }
    }
}

impl Read for Points {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            let read = self.bytes.read(out)?;
            if read != 0 || self.next > self.count {
                return Ok(read);
            }
            let line = if self.next == self.count {
                "ENDSEC;END-ISO-10303-21;".to_owned()
            } else {
                let id = self.next + 1;
                format!("#{id}=CARTESIAN_POINT('point',({id}.,2.,3.));\n")
            };
            self.next += 1;
            self.bytes = io::Cursor::new(line.into_bytes());
        }
    }
}

struct Fingerprint(DefaultHasher);

impl Write for Fingerprint {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.0.write(value.as_bytes());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .ok_or("expected raw|compact and an entity count")?;
    let count: u64 = args.next().ok_or("expected an entity count")?.parse()?;
    if count == u64::MAX || args.next().is_some() {
        return Err("expected raw|compact and a count below u64::MAX".into());
    }
    let mut hash = Fingerprint(DefaultHasher::new());
    let len = match mode.as_str() {
        "raw" => {
            let graph = parse(Points::new(count))?;
            for entity in graph.entities.values() {
                write!(&mut hash, "{entity:?}")?;
            }
            std::hint::black_box(&graph).entities.len()
        }
        "compact" => {
            let graph = parse_compact(Points::new(count))?;
            for id in graph.ids() {
                write!(
                    &mut hash,
                    "{:?}",
                    graph.entity(id).ok_or("indexed entity missing")?
                )?;
            }
            std::hint::black_box(&graph).len()
        }
        _ => return Err("expected raw|compact".into()),
    };
    println!("entities={len} fingerprint={:016x}", hash.0.finish());
    Ok(())
}
