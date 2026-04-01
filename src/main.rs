mod app;
mod clipboard;
mod config;
mod handler;
mod theme;
mod totp;
mod ui;
mod vault;

use app::App;
use config::load_config;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use vault::ensure_vault_dir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" => {
                println!("gmpass {}", VERSION);
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => {
                eprintln!("Unknown option: {}", other);
                eprintln!("Run 'gmpass --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    // Panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    ensure_vault_dir();

    let vault_exists = vault::vault_path().exists();
    let config = load_config();

    let mut app = App::new_locked(vault_exists, config);

    if let Err(e) = run_tui(&mut app) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("gmpass {} - GetMyPass Terminal Password Manager", VERSION);
    println!();
    println!("USAGE:");
    println!("    gmpass [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -V, --version    Print version");
    println!("    -h, --help       Print this help");
    println!();
    println!("KEYBOARD SHORTCUTS:");
    println!("  List view:");
    println!("    n        New password     N        New note");
    println!("    e        Edit entry       d        Delete entry");
    println!("    f        Toggle favorite  D        Duplicate entry");
    println!("    1/c      Copy password    2/u      Copy username");
    println!("    o        Cycle sort       i        View statistics");
    println!("    /        Search           q        Quit");
    println!();
    println!("  Edit mode:");
    println!("    F5       Save             F6       Generate password");
    println!("    Tab      Next field       Esc      Cancel");
    println!();
    println!("  View mode:");
    println!("    p        Reveal password  H        Password history");
    println!("    f        Toggle favorite  e        Edit");
    println!();
    println!("  Navigation:");
    println!("    F1       Vault tab        F2       Settings tab");
    println!("    Tab      Switch tabs      Bksp     Folder up");
    println!();
    println!("FILES:");
    println!("    ~/.gmp/vault.enc       Encrypted vault");
    println!("    ~/.gmp/config.toml     Configuration");
    println!("    ~/.gmp/vault.enc.bak   Automatic backup");
    println!("    ~/.gmp/export.json     Export output");
    println!();
    println!("SECURITY:");
    println!("    AES-256-GCM encryption with Argon2id key derivation.");
    println!("    Atomic writes, memory zeroization, auto-lock.");
}

fn run_tui(app: &mut App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key(app, key);
                if app.should_quit {
                    break;
                }
            }
        }

        // Periodic checks (every tick ~250ms)
        app.clear_expired_status();
        app.check_auto_lock();
        app.check_clipboard_clear();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
