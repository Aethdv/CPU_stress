use anstyle::{AnsiColor, Color, Style};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "locus")]
#[command(version, about = "CPU stress test with memory subsystem pressure", long_about = None)]
pub struct Args {
    #[arg(short, long, default_value_t = 0)]
    pub duration: u64,

    #[arg(short = 'j', long, default_value_t = 0)]
    pub threads: usize,

    #[arg(short, long, default_value = "mixed")]
    #[arg(value_parser = ["integer", "float", "memory", "memory-latency", "memory-bandwidth", "mixed"])]
    pub workload: String,

    /// 0 = auto-detect, overrides -x
    #[arg(short = 'm', long, default_value_t = 0)]
    pub memory_mb: usize,

    /// 2=light, 4=balanced, 8=aggressive, 16=extreme
    #[arg(short = 'x', long, default_value_t = 4)]
    pub memory_multiplier: usize,

    /// Iterations between stop checks
    #[arg(short, long, default_value_t = 100_000)]
    pub batch_size: u64,

    /// Disable progress reporting
    #[arg(short, long)]
    pub quiet: bool,

    /// Run all workloads sequentially
    #[arg(short = 'B', long)]
    pub benchmark: bool,
}

pub fn print_help() {
    let header = Style::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    let cmd = Style::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let opt = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let value = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
    let desc = Style::new();
    let example = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));
    let reset = Style::new();

    println!("{}locus{} {}", cmd, reset, env!("CARGO_PKG_VERSION"));
    println!("A configurable CPU stress testing tool with multiple computational workloads.\n");

    println!("{}USAGE:{}", header, reset);
    println!("    {}locus{} [OPTIONS]\n", cmd, reset);

    println!("{}OPTIONS:{}", header, reset);

    println!(
        "  {}-d{}, {}--duration{} {}SECS{}",
        opt, reset, opt, reset, value, reset
    );
    println!(
        "      {}Duration in seconds (0 = run until Ctrl+C) [default: 0]{}",
        desc, reset
    );

    println!(
        "\n  {}-j{}, {}--threads{} {}NUM{}",
        opt, reset, opt, reset, value, reset
    );
    println!(
        "      {}Number of worker threads (0 = auto-detect all cores) [default: 0]{}",
        desc, reset
    );

    println!(
        "\n  {}-w{}, {}--workload{} {}TYPE{}",
        opt, reset, opt, reset, value, reset
    );
    println!("      {}Workload type: [default: mixed]{}", desc, reset);
    println!(
        "        {}integer         {}{}- Pure CPU integer arithmetic{}",
        value, reset, desc, reset
    );
    println!(
        "        {}float           {}{}- Pure CPU floating-point math{}",
        value, reset, desc, reset
    );
    println!(
        "        {}memory          {}{}- Memory latency test (fallback){}",
        value, reset, desc, reset
    );
    println!(
        "        {}memory-latency  {}{}- Explicit RAM latency test{}",
        value, reset, desc, reset
    );
    println!(
        "        {}memory-bandwidth{}{}- RAM bandwidth saturation{}",
        value, reset, desc, reset
    );
    println!(
        "        {}mixed           {}{}- Integer + float + memory-latency{}",
        value, reset, desc, reset
    );

    println!(
        "\n  {}-m{}, {}--memory-mb{} {}MB{}",
        opt, reset, opt, reset, value, reset
    );
    println!(
        "      {}Memory buffer size in MB (0 = auto-detect, overrides -x) [default: 0]{}",
        desc, reset
    );

    println!(
        "\n  {}-x{}, {}--memory-multiplier{} {}NUM{}",
        opt, reset, opt, reset, value, reset
    );
    println!(
        "      {}Memory multiplier for auto-detection{}",
        desc, reset
    );
    println!(
        "      {}2=light, 4=balanced, 8=aggressive, 16=extreme [default: 4]{}",
        desc, reset
    );

    println!(
        "\n  {}-b{}, {}--batch-size{} {}NUM{}",
        opt, reset, opt, reset, value, reset
    );
    println!(
        "      {}Work batch size (iterations between stop checks) [default: 100000]{}",
        desc, reset
    );

    println!("\n  {}-q{}, {}--quiet{}", opt, reset, opt, reset);
    println!("      {}Disable progress reporting{}", desc, reset);

    println!("\n  {}-B{}, {}--benchmark{}", opt, reset, opt, reset);
    println!(
        "      {}Run all workloads sequentially and display comparison table{}",
        desc, reset
    );

    println!("\n  {}-h{}, {}--help{}", opt, reset, opt, reset);
    println!("      {}Print this help message{}", desc, reset);

    println!("\n  {}-V{}, {}--version{}", opt, reset, opt, reset);
    println!("      {}Print version information{}", desc, reset);

    println!("\n{}EXAMPLES:{}", header, reset);
    println!(
        "  {}# Default balanced stress test for 10 seconds{}",
        example, reset
    );
    println!("  {}locus{} -d 10\n", cmd, reset);

    println!(
        "  {}# Run memory latency workload (pointer-chasing pattern){}",
        example, reset
    );
    println!("  {}locus{} -w memory-latency -d 10 -x 8\n", cmd, reset);

    println!(
        "  {}# Run memory bandwidth workload (parallel streams){}",
        example, reset
    );
    println!("  {}locus{} -w memory-bandwidth -d 10 -x 8\n", cmd, reset);

    println!("  {}# Run full benchmark suite{}", example, reset);
    println!("  {}locus{} --benchmark -d 10 -x 8\n", cmd, reset);

    println!(
        "  {}# Manual memory size override (512 MB per thread){}",
        example, reset
    );
    println!("  {}locus{} -w memory-bandwidth -m 512 -d 10", cmd, reset);
}

pub fn print_version() {
    let cmd = Style::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let reset = Style::new();

    println!("{}locus{} {}", cmd, reset, env!("CARGO_PKG_VERSION"));
}
