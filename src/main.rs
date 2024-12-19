use std::fs;
use std::io;

extern crate clap;
extern crate fastanvil;
extern crate rayon;
extern crate regex;

use clap::Parser;
use fastanvil::Chunk;
use rayon::prelude::*;

pub struct BlockResults {
    pub chunk: (i32, i32, i32),
    pub blocks: Vec<((i32, i32, i32), String)>,
}

pub fn region_coordinates(filename: &str) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let captures = regex::Regex::new(r"r\.(-?\d+).(-?\d+)\.mca")?
        .captures(filename)
        .ok_or("Region file must be in the format r.X.Z.mca")?;
    Ok((
        captures[1].parse::<i32>()? * 32 * 16,
        captures[2].parse::<i32>()? * 32 * 16,
    ))
}

pub fn find_blocks<S: io::Read + io::Seek>(
    filename: &str,
    stream: S,
    block_name: &str,
    chunk_distance_filter: Option<((i32, i32), i32)>,
) -> Result<Vec<BlockResults>, Box<dyn std::error::Error>> {
    let (region_x, region_z) = region_coordinates(filename)?;
    println!("{:>6} {:>6} | {}", region_x, region_z, filename);

    if let Some(((from_x, from_z), maxdist)) = chunk_distance_filter {
        if (region_x - from_x).pow(2) + (region_z - from_z).pow(2) > maxdist.pow(2) {
            return Ok(vec![]);
        }
    }

    let mut region = fastanvil::Region::from_stream(stream)?;

    let mut results: Vec<BlockResults> = vec![];
    for chunk in region.iter().flatten() {
        let complete_chunk = fastanvil::complete::Chunk::from_bytes(&chunk.data)?;
        if complete_chunk.status != "minecraft:full" {
            continue;
        }

        let chunk_x = region_x + (chunk.x as i32) * 16;
        let chunk_z = region_z + (chunk.z as i32) * 16;
        let chunk_y: i32 = complete_chunk.y_range().start as i32;

        if let Some(((from_x, from_z), maxdist)) = chunk_distance_filter {
            if (chunk_x - from_x).pow(2) + (chunk_z - from_z).pow(2) > maxdist.pow(2) {
                continue;
            }
        }

        let found_blocks = complete_chunk
            .iter_blocks()
            .enumerate()
            .filter(|(_, block)| block.name().contains(block_name))
            .map(|(block_index, block)| {
                let x = chunk_x + (block_index as i32) % 16;
                let z = chunk_z + ((block_index as i32) / 16) % 16;
                let y = chunk_y + (block_index as i32) / (16 * 16);
                ((x, y, z), block.name().to_string())
            })
            .collect::<Vec<_>>();

        if !found_blocks.is_empty() {
            results.push(BlockResults {
                chunk: (chunk_x, chunk_y, chunk_z),
                blocks: found_blocks,
            });
        }
    }

    Ok(results)
}

#[derive(serde::Deserialize, Debug, Default)]
pub struct FileConfig {
    pub block: Option<String>,
    pub path: Option<std::path::PathBuf>,
    pub home: Option<(i32, i32)>,
    pub show_all: Option<bool>,
    pub max_distance: Option<i32>,
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Substring of the block name to search for
    block: Option<String>,
    /// Region file directory (e.g. %APPDATA%/.minecraft/saves/world/region)
    #[arg(short, long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    path: Option<std::path::PathBuf>,
    /// Whether to show all blocks rather than only chunks containing them
    #[arg(short, long)]
    show_all: bool,
    /// Whether to show all blocks rather than only chunks containing them
    #[arg(short, long)]
    max_distance: Option<i32>,
}

fn main() {
    let args = Cli::parse();

    let config: FileConfig = {
        if let Ok(config_content) = fs::read_to_string("config.toml") {
            toml::from_str(&config_content).expect("Invalid config file.")
        } else {
            std::default::Default::default()
        }
    };

    let block = args
        .block
        .or(config.block)
        .expect("No block provided. Run as minecraft_block_finder.exe --block \"diamond_ore\" --path \"...\"");
    let home = config.home;
    let path = args
        .path
        .or(config.path)
        .expect("No region path provided (directory of .mca files)");
    let show_all = args.show_all;
    let max_distance = args.max_distance.or(config.max_distance);
    let chunk_distance_filter = max_distance.map(|m| (home.unwrap_or((0, 0)), m));

    let paths: Vec<_> = fs::read_dir(path)
        .expect("Invalid region path.")
        .flatten()
        .map(|x| x.path())
        .collect();
    let results: Vec<BlockResults> = paths
        .par_iter()
        .flat_map(|path| {
            find_blocks(
                path.to_str().unwrap(),
                fs::File::open(path).unwrap(),
                &block,
                chunk_distance_filter,
            )
            .unwrap()
        })
        .collect();

    println!("\n\n\n");
    println!("Found chunks: {}", results.len());
    let mut sorted_results = results;
    if let Some((home_x, home_z)) = home {
        sorted_results.sort_by_key(|r| (r.chunk.0 - home_x).pow(2) + (r.chunk.2 - home_z).pow(2));
    }

    for r in sorted_results {
        if show_all {
            for (b, n) in r.blocks {
                println!("{} {} {} - {}", b.0, b.1, b.2, n)
            }
        } else {
            let block_counts = r
                .blocks
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, (_, b)| {
                    *acc.entry(b.clone()).or_insert(0) += 1;
                    acc
                });
            for (b, count) in block_counts {
                println!("{} {} {} - {} ({})", r.chunk.0, r.chunk.1, r.chunk.2, b, count)
            }
        }
    }
}
