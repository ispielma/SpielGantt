// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;

fn main() {
    if std::env::args_os().len() == 1 {
        spielgantt_lib::run();
        return;
    }

    cli::run();
}
