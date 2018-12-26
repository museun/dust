use std::env;
use std::path::{Path, PathBuf};

use getopts::Options;
use glob::glob;

#[derive(Debug)]
struct Opts {
    name: String,
    reverse: bool,
    percentage: bool,
    path: bool,
    min: f64,
    input: String,
}

impl Opts {
    pub fn parse() -> Self {
        let (name, args) = {
            let mut args = env::args();
            (args.next().unwrap(), args)
        };

        let mut opts = Options::new();
        opts.optflag("h", "help", "shows this help message");
        opts.optflag("r", "reverse", "reverse ordering");
        opts.optflag("P", "percentages", "show percentages");
        opts.optflag("p", "path", "sort by path, instead of by size");
        opts.optopt("m", "min", "show only minimum percentage", "FLOAT");

        let matches = match opts.parse(&args.collect::<Vec<_>>()) {
            Ok(m) => m,
            Err(err) => {
                eprintln!("could not parse args: {}", err);
                std::process::exit(1);
            }
        };

        if matches.opt_present("h") {
            Self::print_usage(&name, &opts)
        };

        Self {
            name,
            reverse: matches.opt_present("r"),
            percentage: matches.opt_present("P"),
            path: matches.opt_present("p"),
            min: matches.opt_get_default("m", 0.00).expect("min percentage"),
            input: matches.free.get(0).cloned().unwrap_or_else(|| "*".into()),
        }
    }

    fn print_usage(name: &str, options: &Options) -> ! {
        let brief = format!("usage: {} [FLAGS] path", name);
        print!("{}", options.usage(&brief));
        std::process::exit(0)
    }
}

fn main() {
    let opts = Opts::parse();
    let dirs = glob(&opts.input).unwrap().filter_map(|p| p.ok());
    let (total_size, total_count, mut entries) = walk_entries(dirs);
    let total_count = format_count(total_count);
    let count_width = total_count.len();

    if opts.path {
        entries.sort_unstable_by(|l, r| l.path.cmp(&r.path))
    } else {
        entries.sort_unstable_by_key(|e| e.size)
    };

    if opts.reverse {
        entries.reverse();
    }

    for entry in entries {
        let p = 100.0 * entry.size as f64 / total_size as f64;
        if p < opts.min {
            continue;
        }

        print!("{:>10} ", format_size(entry.size));
        if opts.percentage {
            print!(" {} ", format!("{:>5.2}%", p));
        }

        print!(" {:>size$} ", format_count(entry.count), size = count_width);

        if entry.path.is_dir() {
            println!(
                " {}{}",
                entry.path.display().to_string(),
                std::path::MAIN_SEPARATOR,
            );
        } else {
            println!(" {}", entry.path.display().to_string());
        }
    }

    let p = if opts.percentage { 8 } else { 0 } + 1;
    use std::iter::repeat;
    println!(
        "{}",
        repeat("-")
            .take(10)
            .chain(repeat(" ").take(p + 1))
            .chain(repeat("-").take(count_width))
            .collect::<String>()
    );

    println!(
        "{:>10} {:>offset$}",
        format_size(total_size),
        total_count,
        offset = p + count_width
    );
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    size: u64,
    count: u64,
}

fn walk_entries<I>(paths: I) -> (u64, u64, Vec<Entry>)
where
    I: IntoIterator<Item = PathBuf>, // TODO figure out how to borrow this as a &'a Path
{
    paths.into_iter().map(|p| (get_sizes(&p), p)).fold(
        (0, 0, vec![]),
        |(total_size, total_count, mut entries), ((size, count), path)| {
            if path.exists() {
                entries.push(Entry { path, size, count })
            }
            (total_size + size, total_count + count, entries)
        },
    )
}

fn get_sizes(path: &Path) -> (u64, u64) {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| {
            e.ok()
                .and_then(|e| e.path().symlink_metadata().ok())
                .map(|d| d.len())
        })
        .fold((0, 0), |(sum, count), c| (sum + c, count + 1))
}

fn format_size(n: u64) -> String {
    const SIZES: [&str; 9] = ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"]; // sure
    let mut order = 0;
    let mut size = n as f64;

    while size >= 1024.0 && order + 1 < SIZES.len() {
        order += 1;
        size /= 1024.0
    }

    format!("{:.2} {}", size, SIZES[order])
}

fn format_count(n: u64) -> String {
    fn comma(n: u64, s: &mut String) {
        if n < 1000 {
            s.push_str(&format!("{}", n));
            return;
        }
        comma(n / 1000, s);
        s.push_str(&format!(",{:03}", n % 1000));
    }
    let mut buf = String::new();
    comma(n, &mut buf);
    buf
}
