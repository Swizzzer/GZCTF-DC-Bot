use colored::*;

pub fn success(msg: impl std::fmt::Display) {
  println!("{}", format!("[+] {}", msg).green());
}

pub fn info(msg: impl std::fmt::Display) {
  println!("{}", format!("[*] {}", msg).blue());
}

pub fn error(msg: impl std::fmt::Display) {
  eprintln!("{}", format!("[-] {}", msg).red());
}
