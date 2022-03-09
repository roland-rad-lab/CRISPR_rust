
extern crate clap;
extern crate csv;
extern crate itertools;
extern crate noodles;

use clap::Parser;
use noodles::bam as bam;
use noodles::sam as sam;

use std::collections;
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::time;

#[derive(Eq, PartialEq, Hash, Clone)]
struct TwoInts
{
    a: bam::record::ReferenceSequenceId,
    b: bam::record::ReferenceSequenceId
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

fn too_many_hits (record: &bam::Record, tag: sam::record::data::field::tag::Tag, n_mismatch: i64)
    -> io::Result<bool>
{
    match record.data ().get (tag) {
        Some(Ok(field)) => field.value().as_int().map(|hits| hits > n_mismatch).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format! ("{} expected integer for tag {:?}, got: {:?}", String::from_utf8 (record.read_name ().to_vec ()).expect ("Record name was invalid UTF-8"), tag, field.value ())
            )
        }),
        Some(Err(e)) => {
            Err (io::Error::new(
                io::ErrorKind::InvalidData,
                format! ("{} error obtaining tag {:?}: {:?}", String::from_utf8 (record.read_name ().to_vec ()).expect ("Record name was invalid UTF-8"), tag, e)
            ))
        },
        None => {
            Err (io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} failed to find {:?} tag", String::from_utf8 (record.read_name ().to_vec ()).expect ("Record name was invalid UTF-8"), tag)
            ))
        }
    }
}

fn fast_read_bam (capacity: usize, bam_file_path: &str, tag_mismatch: sam::record::data::field::tag::Tag, n_mismatch: i64)
    -> ( collections::HashMap<String,collections::HashSet<bam::record::ReferenceSequenceId>>, Vec<String> )
{
    let mut result = collections::HashMap::with_capacity (capacity);

    let mut bam_reader = fs::File::open (bam_file_path).map(bam::Reader::new).expect ("Failed to open bam file");
    let _header: sam::Header = bam_reader.read_header().expect ("Failed to read bam header").parse().expect ("Failed to parse bam header");
    let reference_sequences = bam_reader.read_reference_sequences().expect ("Failed to read reference seuquences");

    for r in bam_reader.records ()
    {
        let record = r.expect ("Failed to parse record");
        let flags = record.flags();

        if !flags.is_unmapped () && !flags.is_secondary () && !too_many_hits (&record, tag_mismatch, n_mismatch).unwrap () {
            result.entry (String::from_utf8 (record.read_name ().to_vec ()).expect ("Failed to convert read name")).or_insert (collections::HashSet::new ()).insert ( record.reference_sequence_id ().expect ("Should be a mapped read with a sequence id") );
        }
    }

    return ( result, reference_sequences.keys ().map (String::from).collect () )
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

    let tag_mismatch = args.tag_mismatch.parse::<sam::record::data::field::tag::Tag> ().expect ("Failed to parse tag");

    let before_read = time::Instant::now ();

    let (read_one_map, read_one_target_names) = fast_read_bam (read_one_bam_capacity, &args.bam_r1, tag_mismatch, args.n_mismatch as i64);
    let (read_two_map, read_two_target_names) = fast_read_bam (read_two_bam_capacity, &args.bam_r2, tag_mismatch, args.n_mismatch as i64);

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

    let before_expand = time::Instant::now ();

    if args.pair
    {
        assert! (read_one_target_names.len ()==read_two_target_names.len (),"Requested pair mode but the number of references does not match");
        for (read_one_tid, read_two_tid) in itertools::zip (0..read_one_target_names.len (), 0..read_two_target_names.len ())
        {
            guide_pairs.entry (TwoInts { a: bam::record::ReferenceSequenceId::from (read_one_tid), b: bam::record::ReferenceSequenceId::from (read_two_tid) }).or_insert (0);
        }
    }
    else
    {
        for (read_one_tid, read_two_tid) in itertools::iproduct! (0..read_one_target_names.len (), 0..read_two_target_names.len ())
        {
            guide_pairs.entry (TwoInts { a: bam::record::ReferenceSequenceId::from (read_one_tid), b: bam::record::ReferenceSequenceId::from (read_two_tid) }).or_insert (0);
        }
    }

    println! ("Done expand Elapsed time: {:.2?}", before_expand.elapsed ());

    let mut wtr = csv::WriterBuilder::new ()
        .delimiter (b'\t')
        .from_path (&args.output_tsv).expect ("Failed to open output tsv");

    wtr.write_record (&["Sample_Name", "R1_hit", "R2_hit", "count"]).expect ("Failed to write output header");

    for (guide_pair, count) in guide_pairs
    {
        let guide_pair_a_name = read_one_target_names[usize::try_from (guide_pair.a).expect ("Failed to convert tid to index, do you still have an unmapped read in bam one?")].clone ();
        let guide_pair_b_name = read_two_target_names[usize::try_from (guide_pair.b).expect ("Failed to convert tid to index, do you still have an unmapped read in bam two?")].clone ();
        wtr.serialize ((&args.sample_name, guide_pair_a_name,guide_pair_b_name,count)).expect ("Failed to write tsv line");
    }
}

