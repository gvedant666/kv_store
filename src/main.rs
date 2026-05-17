use kvstore::server::Server;
use std::env;

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    
    let addr = if args.len() > 1 {
        args[1].clone()
    } else {
        "127.0.0.1:1234".to_string()
    };

    let num_threads = if args.len() > 2 {
        args[2].parse().unwrap_or(4)
    } else {
        4
    };

    println!("Configuration:");
    println!("Address:{}", addr);
    println!("Threads {}", num_threads);

    // Create and run the server
    match Server::bind(&addr) {
        Ok(mut server) => {
            println!("Server started successfully!");
            println!("Press Ctrl+C to stop.\n");
            
            if let Err(e) = server.run() {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    }
}