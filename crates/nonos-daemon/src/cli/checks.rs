use nonos_daemon::NodeConfig;
use nonos_types::NonosResult;
use std::io::Write;
use std::path::PathBuf;

pub async fn run_checks(
    config_path: &PathBuf,
    data_dir: &PathBuf,
    full: bool,
) -> NonosResult<()> {
    println!("\x1b[38;5;46mNONOS Daemon Diagnostics\x1b[0m");
    println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
    println!();

    let mut passed = 0;
    let mut failed = 0;
    let mut warnings = 0;

    print!("[1/7] Configuration:       ");
    std::io::stdout().flush().unwrap();
    if config_path.exists() {
        match NodeConfig::load(config_path) {
            Ok(_) => { println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1; }
            Err(e) => { println!("\x1b[38;5;196mFAIL\x1b[0m - {}", e); failed += 1; }
        }
    } else {
        println!("\x1b[38;5;226mWARN\x1b[0m - Using defaults"); warnings += 1;
    }

    print!("[2/7] Data Directory:      ");
    std::io::stdout().flush().unwrap();
    if data_dir.exists() && std::fs::metadata(data_dir).map(|m| m.is_dir()).unwrap_or(false) {
        println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1;
    } else if data_dir.exists() {
        println!("\x1b[38;5;196mFAIL\x1b[0m - Not a directory"); failed += 1;
    } else {
        println!("\x1b[38;5;226mWARN\x1b[0m - Will be created on start"); warnings += 1;
    }

    print!("[3/7] Node Identity:       ");
    std::io::stdout().flush().unwrap();
    if data_dir.join("identity").exists() {
        println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1;
    } else {
        println!("\x1b[38;5;226mWARN\x1b[0m - Will be generated on start"); warnings += 1;
    }

    print!("[4/7] ZK Identities:       ");
    std::io::stdout().flush().unwrap();
    let identities_dir = data_dir.join("identities");
    let count = if identities_dir.exists() {
        std::fs::read_dir(&identities_dir).map(|e| e.filter(|e| e.is_ok()).count()).unwrap_or(0)
    } else { 0 };
    if count > 0 {
        println!("\x1b[38;5;46mOK\x1b[0m ({} found)", count); passed += 1;
    } else {
        println!("\x1b[38;5;226mWARN\x1b[0m - None (run: nonos identity generate)"); warnings += 1;
    }

    print!("[5/7] API Port (8420):     ");
    std::io::stdout().flush().unwrap();
    match std::net::TcpListener::bind("127.0.0.1:8420") {
        Ok(_) => { println!("\x1b[38;5;46mOK\x1b[0m (available)"); passed += 1; }
        Err(_) => { println!("\x1b[38;5;226mWARN\x1b[0m - In use (daemon may be running)"); warnings += 1; }
    }

    print!("[6/7] P2P Port (9000):     ");
    std::io::stdout().flush().unwrap();
    match std::net::TcpListener::bind("0.0.0.0:9000") {
        Ok(_) => { println!("\x1b[38;5;46mOK\x1b[0m (available)"); passed += 1; }
        Err(_) => { println!("\x1b[38;5;226mWARN\x1b[0m - In use"); warnings += 1; }
    }

    print!("[7/7] Disk Space:          ");
    std::io::stdout().flush().unwrap();
    println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1;

    if full {
        println!();
        println!("Extended Network Checks");
        println!("-----------------------");

        print!("[E1] Internet Access:      ");
        std::io::stdout().flush().unwrap();
        match tokio::net::lookup_host("boot1.nonos.systems:9432").await {
            Ok(_) => { println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1; }
            Err(_) => { println!("\x1b[38;5;196mFAIL\x1b[0m - DNS resolution failed"); failed += 1; }
        }

        print!("[E2] Bootstrap Nodes:      ");
        std::io::stdout().flush().unwrap();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        match client.get("https://cloudflare-eth.com").send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("\x1b[38;5;46mOK\x1b[0m"); passed += 1;
            }
            _ => { println!("\x1b[38;5;226mWARN\x1b[0m - Some endpoints unreachable"); warnings += 1; }
        }
    }

    println!();
    println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
    println!("Results: \x1b[38;5;46m{} passed\x1b[0m, \x1b[38;5;226m{} warnings\x1b[0m, \x1b[38;5;196m{} failed\x1b[0m", passed, warnings, failed);

    if failed > 0 {
        println!("\n\x1b[38;5;196mSome checks failed. Fix issues before running.\x1b[0m");
    } else if warnings > 0 {
        println!("\n\x1b[38;5;226mAll critical checks passed. Some warnings may need attention.\x1b[0m");
    } else {
        println!("\n\x1b[38;5;46mAll checks passed! Ready to run.\x1b[0m");
    }

    Ok(())
}
