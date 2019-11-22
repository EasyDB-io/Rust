//! An interface for working with [easydb.io](https://easydb.io) in rust.
//!
//! # Quick start
//!
//! ```
//! # use std::collections::HashMap;
//! use easydb::EasyDB;
//! 
//! // Create an EasyDB struct to interact with.
//! // Gets information from `./easydb.toml`.
//! let edb: EasyDB = EasyDB::new().unwrap();
//!
//! // Store some data
//!	edb.put("hello", "world").unwrap();
//!	edb.put("goodbye", "earth").unwrap();
//!
//! // Get a single item
//! let stored_hello: String = edb.get("hello").unwrap();
//! assert_eq!(&stored_hello, "world");
//!
//! // Update an item
//! edb.put("goodbye", "dirt").unwrap();
//! assert_eq!(&edb.get("goodbye").unwrap(), "dirt");
//!
//! // Get a HashMap of all database entries
//!	let resp: HashMap<String, String> = edb.list().unwrap();
//!	assert_eq!(&resp["hello"], "world");
//!	assert_eq!(&resp["goodbye"], "dirt");
//!
//! // Delete items
//!	edb.delete("hello").unwrap();
//! let deleted_item: String = edb.get("hello").unwrap();
//! assert_eq!(&deleted_item, "");
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
//! [`parse`](./struct.EasyDB.html#method.from_str) it. For example:
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
//! ## Using the EasyDB
//!
//! The four methods **`get`**, **`put`**, **`delete`**, and **`list`** correspond to the four 
//! available APIs in `easydb.io`. `get` and `delete` take one argument: a key. `put` takes two 
//! arguments: a `key` and a `value`. `list` takes no arguments. Example usage can be seen in the 
//! [Quick start](#quick-start) section at the top of this page.
//!
//! ## Errors
//!
//! All network errors as reported by the `reqwest` crate are returned in `Result`s. Other errors
//! are documented on their respective methods.
//!

use std::{collections::HashMap, fs::read_to_string, io::Write, str::FromStr};

use reqwest::{
	header::{CONTENT_LENGTH, CONTENT_TYPE},
	Client, Url,
};
use serde::{Deserialize, Serialize};
#[macro_use]
extern crate error_chain;

mod errors {
	error_chain! {
		types {
			EdbError, EdbErrorKind, EdbResultExt, EdbResult;
		}
		foreign_links {
			Fs(toml::de::Error);
			Io(std::io::Error);
			Request(reqwest::Error);
			Url(reqwest::UrlError);
			FromUtf8(std::string::FromUtf8Error);
			FromJson(serde_json::Error);
		}
	}
}

pub use errors::{EdbError, EdbErrorKind, EdbResult, EdbResultExt};

#[derive(Debug, Deserialize, Serialize)]
pub struct EasyDB {
	#[serde(rename = "UUID")]
	uuid: String,
	#[serde(rename = "Token")]
	token: String,
	#[serde(skip, default = "Client::new")]
	client: Client,
	#[serde(rename = "URL", default = "default_url")]
	url: String,
}

fn default_url() -> String {
	"https://app.easydb.io/database/".to_string()
}

impl EasyDB {
	/// Creates an EasyDB using the `easydb.toml` in the current directory.
	///
	/// # Errors
	///
	/// Will fail if the file cannot be read (e.g. when it doesn't exist), or if the uuid does not
	/// form a valid URL.
	pub fn new() -> EdbResult<Self> {
		let edb: Self = read_to_string("./easydb.toml")?.parse()?;
		edb.validate_uuid()?;
		Ok(edb)
	}

	/// Creates an EasyDB using a UUID, Token, and optional URL (defaults to
	/// `https://app.easydb.io/database/`).
	///
	/// # Errors
	///
	/// Will fail if `url` or `uuid` don't form a valid URL.
	pub fn from_uuid_token(uuid: String, token: String, url: Option<String>) -> EdbResult<Self> {
		let edb = Self {
			uuid,
			token,
			client: Client::new(),
			url: url.unwrap_or_else(default_url),
		};
		edb.url.parse::<Url>()?;
		edb.validate_uuid()?;
		Ok(edb)
	}

	fn validate_uuid(&self) -> EdbResult<()> {
		self.url.parse::<Url>().unwrap().join(&self.uuid)?;
		Ok(())
	}

	fn create_key_url(&self, key: &str) -> EdbResult<Url> {
		self.url
			.parse::<Url>()
			.unwrap()
			.join(&format!("{}/", self.uuid))
			.unwrap()
			.join(key)
			.chain_err(|| format!("Invalid key: {}", key))
	}

	/// Fetches data associated with `key` and returns it as a `String`. Missing keys are returned
	/// as an empty string.
	///
	/// # Errors
	///
	/// Will fail if the data does not form a valid UTF8 string. If your data is not supposed to,
	/// use [`get_writer`][EasyDB::get_writer].
	pub fn get(&self, key: &str) -> EdbResult<String> {
		let mut s = Vec::new();
		self.get_writer(key, &mut s)?;
		// Values will always be surrounded by quotes.
		Ok(String::from_utf8(s[1..s.len() - 1].to_vec())?)
	}
	/// Writes `value` to `key` and returns the status code.
	pub fn put(&self, key: &str, value: &str) -> EdbResult<u16> {
		let body = format!(r#"{{"value":"{}"}}"#, value);
		Ok(self
			.client
			.post(self.create_key_url(key)?)
			.header(CONTENT_TYPE, "application/json")
			.header(CONTENT_LENGTH, body.len())
			.header("token", &self.token)
			.body(body)
			.send()?
			.status()
			.as_u16())
	}

	/// Deletes data associated with `key` and returns the status code.
	pub fn delete(&self, key: &str) -> EdbResult<u16> {
		Ok(self
			.client
			.delete(self.create_key_url(key)?)
			.header(CONTENT_TYPE, "application/json")
			.header("token", &self.token)
			.send()?
			.status()
			.as_u16())
	}

	/// Returns a HashMap of all the data in this database.
	///
	/// # Errors
	///
	/// Will fail if the data does not form a valid UTF8 string. If your data is not supposed to,
	/// use [`list_writer`][EasyDB::list_writer]. Also will fail if the data is not valid JSON.
	pub fn list(&self) -> EdbResult<HashMap<String, String>> {
		let mut s = Vec::new();
		self.list_writer(&mut s)?;
		Ok(serde_json::from_str(&String::from_utf8(s)?)?)
	}

	/// An alternative to `get()` that works with a writer. Fetches data associated with `key` and
	/// writes into `value`, returning the status code.
	/// 
	/// The response should have the value that was originally sent, surrounded by quotes (`"`). If
	/// the key was never set or has no data, the response will be `""`.
	pub fn get_writer<W>(&self, key: &str, value: &mut W) -> EdbResult<u16>
	where
		W: Write,
	{
		let mut resp = self
			.client
			.get(self.create_key_url(key)?)
			.header("token", &self.token)
			.send()?;
		resp.copy_to(value)?;
		Ok(resp.status().as_u16())
	}
	/// An alternative to `list()` that works with a writer. Fetches all the data in the database
	/// and writes it to `list`, returning the status code.
	pub fn list_writer<W>(&self, list: &mut W) -> EdbResult<u16>
	where
		W: Write,
	{
		let mut resp = self
			.client
			.get(self.url.parse::<Url>().unwrap().join(&self.uuid).unwrap())
			.header("token", &self.token)
			.send()?;
		resp.copy_to(list)?;
		Ok(resp.status().as_u16())
	}
}

impl FromStr for EasyDB {
	type Err = EdbError;

	/// Create an `EasyDB` from a `&str` in the TOML format.
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(toml::from_str(s)?)
	}
}

// Note that in order to run tests, you must create an `easydb.toml` in the current directory.
#[cfg(test)]
mod tests {
	use crate::EasyDB;
	#[test]
	fn edb_from_toml() {
		let s = r#"
			UUID = "abcd"
			Token = "efgh"
		"#;
		let edb: EasyDB = s.parse().unwrap();
		assert_eq!(&edb.uuid, "abcd");
		assert_eq!(&edb.token, "efgh");
	}

	#[test]
	fn list() {
		let edb = EasyDB::new().unwrap();
		edb.put("hello", "world").unwrap();
		edb.put("goodbye", "earth").unwrap();
		assert_eq!(&edb.get("hello").unwrap(), "world");

		let resp = edb.list().unwrap();
		assert_eq!(&resp["hello"], "world");
		assert_eq!(&resp["goodbye"], "earth");

		edb.delete("hello").unwrap();
		edb.delete("goodbye").unwrap();
		let resp = edb.list().unwrap();

		assert!(resp.get("hello").is_none());
		assert!(resp.get("goodbye").is_none());
	}
}
