use std::env;
use std::io::Result as IoResult;
use std::path::Path;

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

fn die(msg: impl AsRef<str>) -> ! {
    eprintln!("error: {}", msg.as_ref());
    std::process::exit(1)
}

fn main() {
    let opts = Opts::parse();
    let dirs: Vec<_> = match glob(&opts.input) {
        Ok(ok) => ok,
        Err(err) => die(&format!(
            "could not use that pattern {}: {}",
            &opts.input, err
        )),
    }
    .collect::<Result<_, _>>()
    .unwrap(); // probably can't happen

    let (total_size, total_count, mut entries) = walk_entries(&dirs);

    let total_count = format_count(total_count);
    let count_width = total_count.len();

    if opts.path {
        entries.sort_unstable_by_key(|e| e.path)
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

    let p = if opts.percentage { 8 } else { 0 };
    let total_size = format_size(total_size);
    let offset = p + count_width + 1;

    use std::iter::repeat;
    println!(
        "{}",
        repeat("-")
            .take(10)
            .chain(repeat(" ").take(p + 2))
            .chain(repeat("-").take(offset - p - 1))
            .collect::<String>()
    );

    println!(
        "{:>10} {:>offset$}",
        total_size,
        total_count,
        offset = offset
    );
}

#[derive(Debug)]
struct Entry<'a> {
    path: &'a Path,
    size: u64,
    count: u64,
}

fn walk_entries<'a, P: AsRef<Path>>(paths: &'a [P]) -> (u64, u64, Vec<Entry<'a>>) {
    paths
        .iter()
        .map(|p| p.as_ref())
        .map(|p| (p, get_sizes(&p)))
        .fold(
            (0, 0, vec![]),
            |(total_size, total_count, mut entries), (path, (size, count))| {
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
