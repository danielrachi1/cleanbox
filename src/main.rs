mod cli;
use cleanbox::process_life_directory_unified;
use cli::parse_args;
use std::process;

fn main() {
    let args = parse_args();

    match process_life_directory_unified(&args.life_path) {
        Ok(result) => {
            println!("\nProcessing completed:");
            println!("  Media files processed: {}", result.media_processed);
            println!("  Documents processed: {}", result.documents_processed);
            println!("  Files skipped: {}", result.files_skipped);
            println!("  Files failed: {}", result.files_failed);
            println!("  Total processed: {}", result.total_processed());

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
