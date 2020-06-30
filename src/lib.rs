#![doc(html_root_url = "https://docs.rs/easydb/0.2.0")]

//! An interface for working with [easydb.io](https://easydb.io) in rust.
//!
//! # Quick start
//!
//! ```
//! # use std::collections::HashMap;
//! # use crate::easydb::errors::EdbError;
//! use easydb::EasyDB;
//!
//! // Create an EasyDB struct to interact with.
//! // Gets information from `./easydb.toml`.
//! let edb: EasyDB = EasyDB::new()?;
//! # edb.clear()?;
//! #
//! # std::thread::sleep(std::time::Duration::from_secs(1));
//!
//! // Store some data
//!	edb.put("hello", "world")?;
//!	edb.put("goodbye", "earth")?;
//! #
//! # std::thread::sleep(std::time::Duration::from_secs(1));
//!
//! // Get a single item
//! let stored_hello: String = edb.get("hello")?;
//! assert_eq!(&stored_hello, "world");
//!
//! // Update an item
//! edb.put("goodbye", "dirt")?;
//! # std::thread::sleep(std::time::Duration::from_secs(1));
//! assert_eq!(&edb.get("goodbye")?, "dirt");
//!
//! // Get a HashMap of all database entries
//!	let resp: HashMap<String, String> = edb.list()?;
//!	assert_eq!(&resp["hello"], "world");
//!	assert_eq!(&resp["goodbye"], "dirt");
//!
//! // Delete items
//!	edb.delete("hello")?;
//! # std::thread::sleep(std::time::Duration::from_secs(1));
//! let deleted_item: String = edb.get("hello")?;
//! assert_eq!(&deleted_item, "");
//! # edb.clear()?;
//! # Ok::<(), EdbError>(())
//! ```
//!
//! # Commands
//!
//! The easiest way to create an [`EasyDB`][EasyDB] is to call [`EasyDB::new()`][EasyDB::new].
//! This generates the struct using data in `./easydb.toml`, which should include the following
//! information:
//!
//! ```toml
//! UUID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
//! Token = "ffffffff-0000-1111-2222-333333333333"
//! URL = "https://app.easydb.io/database/"
//! ```
//!
//! The `URL` field is optional and will default to `https://app.easydb.io/database/`.
//!
//! If your toml is not at the default location, you can just
//! [`parse`](./struct.EasyDB.html#method.from_str) it from a string in toml format. For example:
//!
//! ```
//! # use easydb::EasyDB;
//! let edb: EasyDB = "UUID = \"aaaa...\"\nToken = \"ffff...\"".parse().unwrap();
//! ```
//!
//! If you have individual items, you can initalize an [`EasyDB`][EasyDB] with
//! [`from_uuid_token`][EasyDB::from_uuid_token]:
//!
//! ```
//! # use easydb::EasyDB;
//! let edb = EasyDB::from_uuid_token("aaaa...".to_string(), "ffff...".to_string(), None);
//! ```
//!
//! ## Using EasyDB
//!
//! The four methods [**`get`**][EasyDB::get], [**`put`**][EasyDB::put],
//! [**`delete`**][EasyDB::delete], and [**`list`**][EasyDB::list] correspond to the four available
//! APIs in `easydb.io`. `get` and `delete` take one argument: a key. `put` takes two arguments: a
//! `key` and a `value`. `list` takes no arguments. Example usage can be seen in the
//! [quick start](#quick-start) section at the top of this page.
//!
//! The above methods deal with [`String`](https://doc.rust-lang.org/std/string/struct.String.html) 
//! values and will fail if any value is not a JSON string. If you would like to use JSON, there are 
//! [**`get_json`**][EasyDB::get_json], [**`put_json`**][EasyDB::put_json], and 
//! [**`list_json`**][EasyDB::list_json] ([**`delete`**][EasyDB::delete] is the same). These deal 
//! with `value`s that are of the `Json` type, which is a re-export of the 
//! [`Value`](https://docs.serde.rs/serde_json/enum.Value.html) type from `serde_json`.
//!
//! In addition, there is the [**`clear`**][EasyDB::clear] method for easily clearing the entire
//! database, which, for example, is useful when initializing the database. This just calls
//! [`delete`][EasyDB::delete] on every item, but if easydb.io implements a clear function in the
//! future, this will call it.
//!
//! ## Errors
//!
//! All network errors as reported by the `reqwest` crate are returned in `Result`s. Other errors
//! are documented on their respective methods.
//!
//! Due to the unknown nature of the database, there may be unexpected results when reading data
//! just after writing data. Expect that read values will be either up-to-date or old values.
//!

mod easydb;
pub use crate::easydb::EasyDB;

/// Re-exported [`Value`](https://docs.serde.rs/serde_json/enum.Value.html) type from serde_json.
pub use crate::easydb::Json;

pub mod errors;

// Note that in order to run tests, you must create an `easydb.toml` in the current directory.
#[cfg(test)]
mod tests {
	use crate::{errors::EdbResult, EasyDB};
	use serde_json::json;
	#[test]
	fn list() -> EdbResult<()> {
		let edb = EasyDB::new()?;
		edb.clear()?;
		edb.put("hello", "world")?;
		edb.put("goodbye", "earth")?;
		std::thread::sleep(std::time::Duration::from_secs(1));
		assert_eq!(&edb.get("hello")?, "world");
		let list = edb.list()?;
		assert_eq!(&list["hello"], "world");
		assert_eq!(&list["goodbye"], "earth");
		edb.delete("hello")?;
		edb.delete("goodbye")?;
		std::thread::sleep(std::time::Duration::from_secs(1));
		let list = edb.list()?;
		assert!(list.get("hello").is_none());
		assert!(list.get("goodbye").is_none());
		Ok(())
	}
	#[test]
	fn list_json() -> EdbResult<()> {
		let edb = EasyDB::new()?;
		edb.clear()?;
		edb.put_json("hello", json!("world"))?;
		edb.put_json(
			"goodbye",
			json!({
				"a": "b",
				"c": ["d", "e"]
			}),
		)?;
		std::thread::sleep(std::time::Duration::from_secs(1));
		let list = edb.list_json()?;
		assert_eq!(list["hello"], json!("world"));
		assert_eq!(
			list["goodbye"],
			json!({
				"a": "b",
				"c": ["d", "e"]
			})
		);
		Ok(())
	}
}
