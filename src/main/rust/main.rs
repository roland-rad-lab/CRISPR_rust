
extern crate csv;
extern crate itertools;
extern crate rust_htslib;

use rust_htslib::{bam, bam::Read, bam::record::Aux};

use std::collections;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::process;
use std::time;



#[derive(Eq, PartialEq, Hash, Clone)]
struct TwoInts {
    a: i32,
    b: i32
}

fn fast_read_bam (capacity: usize, bam_file_path: &str, n_mismatch: u32)
    -> ( collections::HashMap<String,collections::HashSet<i32>>, Vec<Vec<u8>> )
{
    let mut result = collections::HashMap::with_capacity (capacity);

    let mut bam = bam::Reader::from_path (bam_file_path).unwrap ();
    let mut record = bam::Record::new ();
    
    while let Some(r) = bam.read (&mut record)
    {
        r.expect("Failed to parse record");

        if !record.is_unmapped () {
            match record.aux(b"XM") {
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
                    println! ("failed to find XM tag {:?}", e);
                }
            }
        }
    }

    return ( result, bam.header ().target_names ().iter ().map (|x| x.to_vec ()).collect () )
}

fn main ()
{
    //println! ("hello");

    let args: Vec<String> = env::args ().collect ();
    if args.len () < 6 {
        println! ("Please supply output tsv, sample name, number of mismatches, read one bam file path, read two bam file path");
        process::exit (1);
    }

    if !args[1].clone ().ends_with (".tsv") || args[1].clone ().ends_with (".bam") {
        panic! ("Please supply arguments in the correct order");
    }

    let n_mismatch = args[3].parse::<u32>().expect ("Please supply a valid integer as n_mismatches");

    let read_one_bam_size = fs::metadata (args[4].clone ()).expect ("Failed to read size of bam_one").len ();
    let read_two_bam_size = fs::metadata (args[5].clone ()).expect ("Failed to read size of bam_two").len ();

    //println! ("read_one_bam_size: {}",read_one_bam_size);
    //println! ("read_two_bam_size: {}",read_two_bam_size);

    let read_one_bam_capacity = (read_one_bam_size / 20) as usize;
    let read_two_bam_capacity = (read_two_bam_size / 20) as usize;

    println! ("read_one_bam_capacity: {}",read_one_bam_capacity);
    println! ("read_two_bam_capacity: {}",read_two_bam_capacity);
    println! ("Returning reads with {:?} mismatches", n_mismatch);

    let before_read = time::Instant::now ();

    let (read_one_map, read_one_target_names) = fast_read_bam (read_one_bam_capacity, &args[4], n_mismatch);
    let (read_two_map, read_two_target_names) = fast_read_bam (read_two_bam_capacity, &args[5], n_mismatch);

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

    for (read_one_tid, read_two_tid) in itertools::iproduct! (0..read_one_target_names.len (), 0..read_two_target_names.len ())
    {
        guide_pairs.entry (TwoInts { a: read_one_tid as i32, b: read_two_tid as i32}).or_insert (0);
    }

    let mut wtr = csv::WriterBuilder::new ()
        .delimiter (b'\t')
        .from_path (args[1].clone ()).expect ("Failed to open output tsv");

    wtr.write_record (&["Sample_Name", "R1_hit", "R2_hit", "count"]).expect ("Failed to write output header");

    for (guide_pair, count) in guide_pairs
    {
        let guide_pair_a_name = String::from_utf8 (read_one_target_names[usize::try_from (guide_pair.a).expect ("Failed to convert tid to index, do you still have an unmapped read in bam one?")].clone ()).expect ("Failed to covert read one target name");
        let guide_pair_b_name = String::from_utf8 (read_two_target_names[usize::try_from (guide_pair.b).expect ("Failed to convert tid to index, do you still have an unmapped read in bam two?")].clone ()).expect ("Failed to covert read one target name");
        wtr.serialize ((args[2].clone (), guide_pair_a_name,guide_pair_b_name,count)).expect ("Failed to write tsv line");
    }
}

