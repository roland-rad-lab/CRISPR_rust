
extern crate clap;
extern crate csv;
extern crate itertools;
extern crate rust_htslib;

use clap::Parser;
use rust_htslib::{bam, bam::Read, bam::record::Aux};


use std::collections;
use std::convert::TryFrom;
use std::fs;
use std::time;

#[derive(Eq, PartialEq, Hash, Clone)]
struct TwoInts
{
    a: i32,
    b: i32
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args
{
    #[clap(short, long, default_value = "counts.tsv")]
    output_tsv: String,

    #[clap(short, long)]
    pair: bool,

    #[clap(short, long, parse(try_from_str), default_value = "2")]
    n_mismatch: u32,

    #[clap(short, long, default_value = "NM")]
    tag_mismatch: String,

    sample_name: String,
    bam_r1: String,
    bam_r2: String
}

fn fast_read_bam (capacity: usize, bam_file_path: &str, tag_mismatch: &[u8], n_mismatch: u32)
    -> ( collections::HashMap<String,collections::HashSet<i32>>, Vec<Vec<u8>> )
{
    let mut result = collections::HashMap::with_capacity (capacity);

    let mut bam = bam::Reader::from_path (bam_file_path).unwrap ();
    let mut record = bam::Record::new ();

    while let Some(r) = bam.read (&mut record)
    {
        r.expect("Failed to parse record");

        if !record.is_unmapped () && !record.is_secondary () {
            match record.aux (tag_mismatch) {
                Ok(value) => {
                    // Typically, callers expect an aux field to be of a certain type.
                    // If that's not the case, the value can be `match`ed exhaustively.
                    if let Aux::U8 (v) = value {
                        if u32::from (v) <= n_mismatch {
                            result.entry (String::from_utf8 (record.qname ().to_vec ()).expect ("Failed to convert read name")).or_insert (collections::HashSet::new ()).insert ( record.tid ());
                        }
                    }
                },
                Err(e) => {
                    panic! ("{} failed to find {} tag {:?}", String::from_utf8 (record.qname ().to_vec ()).expect ("Record name was invalid UTF-8"), String::from_utf8 (tag_mismatch.to_vec ()).expect("tag_mismatch was invalid UTF-8"), e);
                }
            }
        }
    }

    return ( result, bam.header ().target_names ().iter ().map (|x| x.to_vec ()).collect () )
}

fn main ()
{
    let args = Args::parse();

    let read_one_bam_size = fs::metadata (&args.bam_r1).expect ("Failed to read size of bam_one").len ();
    let read_two_bam_size = fs::metadata (&args.bam_r2).expect ("Failed to read size of bam_two").len ();

    //println! ("read_one_bam_size: {}",read_one_bam_size);
    //println! ("read_two_bam_size: {}",read_two_bam_size);

    let read_one_bam_capacity = (read_one_bam_size / 20) as usize;
    let read_two_bam_capacity = (read_two_bam_size / 20) as usize;

    println! ("read_one_bam_capacity: {}",read_one_bam_capacity);
    println! ("read_two_bam_capacity: {}",read_two_bam_capacity);
    println! ("Returning reads with {:?} mismatches", args.n_mismatch);

    let before_read = time::Instant::now ();

    let (read_one_map, read_one_target_names) = fast_read_bam (read_one_bam_capacity, &args.bam_r1, &args.tag_mismatch.as_bytes (), args.n_mismatch);
    let (read_two_map, read_two_target_names) = fast_read_bam (read_two_bam_capacity, &args.bam_r2, &args.tag_mismatch.as_bytes (), args.n_mismatch);

    println! ("Done read pairs Elapsed time: {:.2?}", before_read.elapsed ());

    println! ("read_one_map: {:?}",read_one_map.len ());
    println! ("read_two_map: {:?}",read_two_map.len ());

    let before_pair = time::Instant::now ();

    let mut guide_pairs = collections::HashMap::new ();

    for qname in read_one_map.keys ().cloned ().collect::<collections::HashSet<String>> ().intersection (&read_two_map.keys ().cloned ().collect::<collections::HashSet<String>> ())
    {
        for read_one_tid in &read_one_map[qname]
        {
            for read_two_tid in &read_two_map[qname]
            {
                *guide_pairs.entry (TwoInts { a: *read_one_tid, b: *read_two_tid}).or_insert (0) += 1
            }
        }
    }

    println! ("Done make pairs Elapsed time: {:.2?}", before_pair.elapsed ());

    if args.pair
    {
        assert! (read_one_target_names.len ()==read_two_target_names.len (),"Requested pair mode but the number of references does not match");
        for (read_one_tid, read_two_tid) in itertools::zip (0..read_one_target_names.len (), 0..read_two_target_names.len ())
        {
            guide_pairs.entry (TwoInts { a: read_one_tid as i32, b: read_two_tid as i32}).or_insert (0);
        }
    }
    else
    {
        for (read_one_tid, read_two_tid) in itertools::iproduct! (0..read_one_target_names.len (), 0..read_two_target_names.len ())
        {
            guide_pairs.entry (TwoInts { a: read_one_tid as i32, b: read_two_tid as i32}).or_insert (0);
        }
    }

    let mut wtr = csv::WriterBuilder::new ()
        .delimiter (b'\t')
        .from_path (&args.output_tsv).expect ("Failed to open output tsv");

    wtr.write_record (&["Sample_Name", "R1_hit", "R2_hit", "count"]).expect ("Failed to write output header");

    for (guide_pair, count) in guide_pairs
    {
        let guide_pair_a_name = String::from_utf8 (read_one_target_names[usize::try_from (guide_pair.a).expect ("Failed to convert tid to index, do you still have an unmapped read in bam one?")].clone ()).expect ("Failed to covert read one target name");
        let guide_pair_b_name = String::from_utf8 (read_two_target_names[usize::try_from (guide_pair.b).expect ("Failed to convert tid to index, do you still have an unmapped read in bam two?")].clone ()).expect ("Failed to covert read one target name");
        wtr.serialize ((&args.sample_name, guide_pair_a_name,guide_pair_b_name,count)).expect ("Failed to write tsv line");
    }
}

