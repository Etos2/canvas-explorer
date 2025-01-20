use std::fs::OpenOptions;
use std::io::{stdin, BufReader, BufWriter};

use anyhow::{Result, anyhow, bail};
use io::pxls::PxlsFile;
use mctc_canvas_base::{CanvasBaseCodec, CanvasEvent, CanvasMeta, MetaId, PaletteChunk, Placement};
use mctc_parser::Codec;
use mctc_parser::data::{Header, Record};
use mctc_parser::writer::{write_header, write_record};

pub mod io;

fn main() -> Result<()> {
    let mut header = Header::default();
    let id = header
        .register_codec::<CanvasBaseCodec>()
        .ok_or(anyhow!("failed to register codec"))?;
    let mut codec = CanvasBaseCodec::new(id);

    let destination = std::env::var("HOME").unwrap().to_string() + "/pxls/out/c86.mctc";
    eprintln!("Opening file... {}", destination);
    let mut wtr = BufWriter::new(OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(destination)?);

    eprintln!("Reading stdin...");
    let input = stdin();
    let file = PxlsFile::read_from(BufReader::new(input))?;
    if file.lines().is_empty() {
        bail!("file is empty");
    }
    eprintln!("Read {} lines!", { file.lines().len() });

    let time_start = file.lines().first().unwrap().time;
    let time_end = file.lines().last().unwrap().time;
    let mut size = (0, 0);
    for line in file.iter() {
        size.0 = size.0.max(line.pos.0 + 1);
        size.1 = size.1.max(line.pos.1 + 1);
    }

    eprintln!("Size {:?}", size);
    eprintln!("Duration ({} ms)", time_end - time_start);
    let meta = CanvasMeta {
        size,
        time_start,
        time_end: Some(time_end),
        name: "c86".to_string(),
        platform: "pxls.space".to_string(),
    };

    // TODO: Better write api
    eprintln!("Writing...");
    let now = std::time::SystemTime::now();
    write_header(&mut wtr, &header)?;

    codec.write_record(&mut wtr, &CanvasEvent::CanvasMeta(meta))?;
    codec.write_record(
        &mut wtr,
        &CanvasEvent::PaletteChunk(PaletteChunk {
            offset: 0,
            colors: vec![
                [0x00, 0x00, 0x00, 0x00], // Transparent
                [0xFF, 0xFF, 0xFF, 0xFF], // Light Grey
                [0xb9, 0xb3, 0xcf, 0xFF], // Medium Grey
                [0x77, 0x7f, 0x8c, 0xFF], // Dark Grey
                [0x00, 0x00, 0x00, 0xFF], // Black
                [0x38, 0x22, 0x15, 0xFF], // Dark Chocolate
                [0x7c, 0x3f, 0x20, 0xff], // Chocolate
                [0xc0, 0x6f, 0x37, 0xff], // Brown
                [0xfe, 0xad, 0x6c, 0xff], // Peach
                [0xff, 0xd2, 0xb1, 0xff], // Beige
                [0xff, 0xa4, 0xd0, 0xff], // Pink
                [0xf1, 0x4f, 0xb4, 0xff], // Magenta
                [0xe9, 0x73, 0xff, 0xff], // Mauve
                [0xa6, 0x30, 0xd2, 0xff], // Purple
                [0x53, 0x1d, 0x8c, 0xff], // Dark Purple
                [0x24, 0x23, 0x67, 0xff], // Navy
                [0x03, 0x34, 0xbf, 0xff], // Blue
                [0x14, 0x9c, 0xff, 0xff], // Azure
                [0x8d, 0xf5, 0xff, 0xff], // Aqua
                [0x01, 0xbf, 0xa5, 0xff], // Light Teal
                [0x16, 0x77, 0x7e, 0xff], // Dark Teal
                [0x05, 0x45, 0x23, 0xff], // Forest
                [0x18, 0x86, 0x2f, 0xff], // Dark Green
                [0x61, 0xe0, 0x21, 0xff], // Green
                [0xb1, 0xff, 0x37, 0xff], // Lime
                [0xff, 0xff, 0xa5, 0xff], // Pastel Yellow
                [0xfd, 0xe1, 0x11, 0xff], // Yellow
                [0xff, 0x9f, 0x17, 0xff], // Orange
                [0xf6, 0x6e, 0x08, 0xff], // Rust
                [0x55, 0x00, 0x22, 0xff], // Maroon
                [0x99, 0x01, 0x1a, 0xff], // Rose
                [0xf3, 0x0f, 0x0c, 0xff], // Red
                [0xff, 0x78, 0x72, 0xff], // Watermelon
            ],
        }),
    )?;

    for line in file.iter() {
        let color_index = if line.index == 255 { 0 } else { line.index + 1 };
        let place = Placement {
            pos: line.pos,
            time: line.time,
            color_index,
        };
        let id = MetaId::Numerical(line.id.as_str().as_bytes().to_vec());
        codec.write_record(&mut wtr, &CanvasEvent::Placement(place))?;
        codec.write_record(&mut wtr, &CanvasEvent::MetaId(id))?;
    }

    // eos
    write_record(&mut wtr, &Record::new_eos())?;
    eprintln!("End of stream!");
    eprintln!("Write took {} ms", now.elapsed()?.as_millis());
    Ok(())
}
