extern crate baffles;

use baffles::bloom::*;
use baffles::standard::DefaultStandardBloom;
use baffles::blocked::DefaultBlockedBloom;

#[derive(Debug)]
struct RunResult {
    name: String,

    n: usize,
    c: usize,
    k: usize,

    false_positives: usize,
}

fn main() {
    let n = 10 * 1024;
    let c = 12;
    let k = optimal_hashers(c);
    let b = 16;

    let runs = 20;

    let def_standard_runs = (0..runs).map(|_| run(&mut DefaultStandardBloom::new(n, c, k)));
    let def_blocked_runs = (0..runs).map(|_| run(&mut DefaultBlockedBloom::new(n, c, k, b)));

    let c = def_standard_runs.chain(def_blocked_runs);

    for r in c {
        let fp = r.false_positives as f64 / r.n as f64;
        let fpp = false_positive_probability(r.n, r.c, r.k);

        let abs_diff = fp - fpp;

        println!(
            "{:>10}: {:5} out of {} checks were false positives. \
             This rate is {:.7} with an expected rate of {:.7}. (diff: {:>+12.7})",
            r.name,
            r.false_positives,
            r.n,
            fp,
            fpp,
            abs_diff,
        );
    }
}

fn run<B: BloomFilter<usize>>(bf: &mut B) -> RunResult {
    let n = bf.set_size();
    let c = bf.bits_per_member();
    let k = bf.hash_count();

    let mut marked = 0;
    let mut i = 0;

    // Insert `n` items into the filter.
    loop {
        if !bf.check(&i) {
            // Only insert `i` if it's not already marked in the
            // filter. This can happen when we get false positives
            // before marking all `n` items.
            bf.mark(&i);
            marked += 1;

            // When we've marked `n` items, we're done here.
            if marked >= n {
                break;
            }
        }
        i += 1;
    }

    let not_members_start = i + 1;
    let not_members_end = i + n;
    let false_positives = (not_members_start..not_members_end).fold(0, |acc, v| {
        if bf.check(&v) { acc + 1 } else { acc }
    });

    RunResult {
        name: bf.name().to_string(),
        n: n,
        c: c,
        k: k,
        false_positives: false_positives,
    }
}
