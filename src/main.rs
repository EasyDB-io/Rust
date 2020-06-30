//! An example usage of easydb using an interactive prompt

mod easydb;
use crate::easydb::EasyDB;
use std::{
	env::args,
	io::{stdin, stdout, Write},
};
mod errors;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let edb;
	let args: Vec<_> = args().collect();
	match args.len() {
		1 => loop {
			match EasyDB::new() {
				Ok(x) => {
					edb = x;
					break;
				}
				Err(e) => {
					eprintln!("{}", e);
					eprintln!("Make sure `easydb.toml` exists, then press enter.");
					input();
				}
			}
		},
		3 => edb = { EasyDB::from_uuid_token(args[1].clone(), args[2].clone(), None)? },
		4 => {
			edb = EasyDB::from_uuid_token(args[1].clone(), args[2].clone(), Some(args[3].clone()))?
		}
		_ => {
			eprintln!("Invalid args, accepts 0, 2, or 3 arguments: [<UUID> <Token> [URL]]");
			std::process::exit(1);
		}
	}
	println!("EasyDB interactive prompt");
	println!("-----------------------------------");
	println!("    Commands:");
	println!("    get      Get a value by key");
	println!("    put      Set a key to a value");
	println!("    del      Delete an item by key");
	println!("    list     List all items in DB");
	println!("    clear    Delete all items");
	println!("    uuid     Get UUID");
	println!("    token    Get token");
	println!("    url      Get URL");
	println!("    exit     Exit the program");
	println!();
	loop {
		print!("> ");
		stdout().flush()?;
		match &input()[..] {
			"get" => {
				print!("Key:");
				stdout().flush()?;
				println!("{}", edb.get(&input())?);
			}
			"put" => {
				print!("Key:");
				stdout().flush()?;
				let key = input();
				print!("Value:");
				stdout().flush()?;
				println!("Code: {}", edb.put(&key, &input())?);
			}
			"del" => {
				print!("Key:");
				stdout().flush()?;
				println!("Code: {}", edb.delete(&input())?);
			}
			"list" => {
				for (key, val) in edb.list()?.drain() {
					println!("{}: {}", key, val);
				}
			}
			"clear" => {
				edb.clear()?;
				println!("Success");
			}
			"uuid" => {
				println!("{}", edb.uuid());
			}
			"token" => {
				println!("{}", edb.token());
			}
			"url" => {
				println!("{}", edb.url());
			}
			"exit" => {
				break;
			}
			_ => println!("Invalid command."),
		}
	}
	Ok(())
}

fn input() -> String {
	let mut s = String::new();
	stdin().read_line(&mut s).unwrap();
	s.trim().to_string()
}
