mod app;
mod clipboard;
mod handler;
mod ui;
mod vault;

use app::App;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use vault::{ensure_vault_dir, load_vault};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Handle --version flag
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("gmp {}", VERSION);
        return;
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

    if !vault_exists {
        println!("Welcome to GetMyPass!");
        println!("No vault found. Creating a new one at ~/.gmp/vault.enc");
        println!();
    }

    // Get master password
    let master_password = if vault_exists {
        prompt_unlock()
    } else {
        prompt_new_password()
    };

    // Load vault
    let vault = match load_vault(&master_password) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut app = App::new(vault, master_password);

    // If it's a new vault, save it immediately to create the file
    if !vault_exists {
        if let Err(e) = vault::save_vault(&app.vault, &app.master_password) {
            eprintln!("Error creating vault: {}", e);
            std::process::exit(1);
        }
    }

    // Run TUI
    if let Err(e) = run_tui(&mut app) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn prompt_unlock() -> String {
    for attempt in 0..3 {
        let password = rpassword::prompt_password("Master password: ").unwrap_or_default();
        if password.is_empty() {
            eprintln!("Password cannot be empty.");
            continue;
        }
        // Try to decrypt
        match load_vault(&password) {
            Ok(_) => return password,
            Err(_) => {
                if attempt < 2 {
                    eprintln!("Wrong password. Try again ({}/3).", attempt + 2);
                } else {
                    eprintln!("Too many failed attempts.");
                    std::process::exit(1);
                }
            }
        }
    }
    unreachable!()
}

fn prompt_new_password() -> String {
    loop {
        let password =
            rpassword::prompt_password("Choose a master password: ").unwrap_or_default();
        if password.len() < 4 {
            eprintln!("Password must be at least 4 characters.");
            continue;
        }
        let confirm =
            rpassword::prompt_password("Confirm master password: ").unwrap_or_default();
        if password != confirm {
            eprintln!("Passwords do not match. Try again.");
            continue;
        }
        return password;
    }
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

        app.clear_expired_status();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
