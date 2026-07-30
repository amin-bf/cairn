//! `fsrsbench [sizes] [cards_per_item] [seed] [eval_below]`
//!
//! Pushed to the handset with `adb push` and run under `adb shell`; also the desktop
//! reference run. Defaults match the ladder in the issue-20 write-up.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sizes: Vec<usize> = match args.first() {
        Some(spec) => spec
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect(),
        None => vec![5_000, 20_000, 73_000, 250_000, 730_000],
    };
    let cards_per_item: f64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(0.05);
    let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(20);
    let eval_below: usize = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(100_000);

    print!("{}", fsrsbench::run(&sizes, cards_per_item, seed, eval_below));
}
