mod cli;
use cleanbox::process_media_directory;
use cli::parse_args;
use std::path::Path;
use std::process;

fn main() {
    let args = parse_args();
    let inbox_path = format!("{}/inbox", args.life_path.trim_end_matches('/'));
    let media_root = Path::new(&args.life_path).join("media");

    match process_media_directory(&inbox_path, &media_root) {
        Ok(result) => {
            println!("Processing completed:");
            println!("  Processed: {} files", result.processed_files);
            println!("  Skipped: {} files", result.skipped_files);
            println!("  Failed: {} files", result.failed_files);

            if !result.errors.is_empty() {
                println!("\nErrors:");
                for error in &result.errors {
                    eprintln!("  {error}");
                }
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to process directory: {e}");
            process::exit(1);
        }
    }
}
