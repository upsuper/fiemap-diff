use anyhow::Result;
use fiemap::FiemapExtent;
use humansize::{file_size_opts::BINARY, FileSize};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::u64;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    file1: PathBuf,
    #[structopt(parse(from_os_str))]
    file2: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let mut extents1 = get_sorted_physical_ranges(&opt.file1)?.into_iter();
    let mut extents2 = get_sorted_physical_ranges(&opt.file2)?.into_iter();
    let mut extent1 = extents1.next();
    let mut extent2 = extents2.next();
    let mut diff_bytes1 = 0;
    let mut diff_bytes2 = 0;
    let mut shared_bytes = 0;
    loop {
        match (&mut extent1, &mut extent2) {
            (Some(e1), Some(e2)) => {
                if e1.end <= e2.start {
                    // e1 is before e2
                    diff_bytes1 += e1.end - e1.start;
                    extent1 = extents1.next();
                } else if e1.start >= e2.end {
                    // e1 is after e2
                    diff_bytes2 += e2.end - e2.start;
                    extent2 = extents2.next();
                } else {
                    // Otherwise, two extents intersect.
                    // Align the start of the two extents.
                    if e1.start < e2.start {
                        diff_bytes1 += e2.start - e1.start;
                        e1.start = e2.start;
                    } else if e1.start > e2.start {
                        diff_bytes2 += e1.start - e2.start;
                        e2.start = e1.start;
                    }
                    // Count the shared part of the two extents.
                    if e1.end < e2.end {
                        shared_bytes += e1.end - e1.start;
                        e2.start = e1.end;
                        extent1 = extents1.next();
                    } else if e1.end > e2.end {
                        shared_bytes += e2.end - e2.start;
                        e1.start = e2.end;
                        extent2 = extents2.next();
                    } else {
                        shared_bytes += e1.end - e1.start;
                        extent1 = extents1.next();
                        extent2 = extents2.next();
                    }
                }
            }
            (Some(e1), None) => {
                diff_bytes1 += e1.end - e1.start;
                extent1 = extents1.next();
            }
            (None, Some(e2)) => {
                diff_bytes2 += e2.end - e2.start;
                extent2 = extents2.next();
            }
            (None, None) => break,
        }
    }

    let diff_bytes1 = diff_bytes1.file_size(BINARY).unwrap();
    let diff_bytes2 = diff_bytes2.file_size(BINARY).unwrap();
    let shared_bytes = shared_bytes.file_size(BINARY).unwrap();
    println!("{}: {} unique", opt.file1.display(), diff_bytes1);
    println!("{}: {} unique", opt.file2.display(), diff_bytes2);
    println!("{} shared", shared_bytes);

    Ok(())
}

fn get_sorted_physical_ranges(path: &Path) -> Result<Vec<Range<u64>>> {
    let mut result = fiemap::fiemap(path)?
        .map(|extent| {
            let FiemapExtent {
                fe_physical: offset,
                fe_length: length,
                ..
            } = extent?;
            Ok(offset..offset + length)
        })
        .collect::<Result<Vec<_>>>()?;
    result.sort_unstable_by_key(|range| (range.start, range.end));
    Ok(result)
}
