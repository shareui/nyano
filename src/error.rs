use owo_colors::OwoColorize;

pub fn print_error(message: &str) {
    eprintln!("{}", format!("Error: {message}").red());
}
