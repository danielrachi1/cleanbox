mod cli;
use cleanbox::rename_all_media_in_dir;
use cli::parse_args;
use std::path::Path;

fn main() {
    let args = parse_args();
    let inbox_path = format!("{}/inbox", args.life_path.trim_end_matches('/'));
    let media_root = Path::new(&args.life_path).join("media");
    rename_all_media_in_dir(&inbox_path, &media_root);
}
