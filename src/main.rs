mod cli;
use cleanbox::process_life_directory;
use cli::parse_args;
use std::process;

fn main() {
    let args = parse_args();

    match process_life_directory(&args.life_path) {
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
