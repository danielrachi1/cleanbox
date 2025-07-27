mod cli;
use cli::parse_args;
use cleanbox::{rename_all_media_in_dir};

fn main() {
    let args = parse_args();
    let inbox_path = format!("{}/inbox", args.life_path.trim_end_matches('/'));
    rename_all_media_in_dir(&inbox_path);
}
