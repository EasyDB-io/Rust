use crate::errors::{EdbError, EdbResult, EdbResultExt};
use reqwest::{
	header::{CONTENT_LENGTH, CONTENT_TYPE},
	Client, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
pub use serde_json::Value as Json;
use std::{collections::HashMap, fs::read_to_string, io::Write, str::FromStr};
#[derive(Debug, Deserialize, Serialize)]

/// The main type for dealing with easydb.
///
/// Create an `EasyDB` using [`new`][EasyDB::new] or [`from_uuid_token`][EasyDB::from_uuid_token].
///
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
	/// Will fail if the file cannot be read (e.g. when it doesn't exist), or if the UUID or URL
	/// does not form a valid URL.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// let edb = EasyDB::new()?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
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
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// let edb = EasyDB::from_uuid_token("aaaa...".to_string(), "bbbb...".to_string(), None);
	/// # Ok::<(), EdbError>(())
	/// ```
	///
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
	/// Returns the stored UUID.
	pub fn uuid(&self) -> &str {
		&self.uuid
	}
	/// Returns the stored token.
	pub fn token(&self) -> &str {
		&self.token
	}
	/// Returns the stored URL.
	pub fn url(&self) -> &str {
		&self.url
	}

	/// Gets the value associated with `key`.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let s = edb.get("somekey")?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn get(&self, key: &str) -> EdbResult<String> {
		self.get_json(key)?
			.as_str()
			.map(|s| s.to_string())
			.ok_or_else(|| "Value was not a string".into())
	}
	/// Gets the value associated with `key` in json format.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let json = edb.get_json("somekey")?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn get_json(&self, key: &str) -> EdbResult<Json> {
		let mut s = Vec::new();
		self.get_writer(key, &mut s)?;
		Ok(serde_json::from_slice(&s)?)
	}
	/// Assigns `value` to `key` and returns the status code.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let status = edb.put("somekey", "somevalue")?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn put(&self, key: &str, value: &str) -> EdbResult<u16> {
		let new_value = json!(value);
		self.put_json(key, new_value)
	}
	/// Assigns a json `value` to `key` and returns the status code.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # use serde_json::json;
	/// # let edb = EasyDB::new()?;
	/// let status = edb.put_json("somekey", json!({"a": "b"}))?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn put_json(&self, key: &str, value: Json) -> EdbResult<u16> {
		let body = json!({ "value": value }).to_string();
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
	/// Deletes the value associated with `key` and returns the status code.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let status = edb.delete("somekey")?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
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
	/// Returns a `HashMap<String, String>` of all the data in this database.
	///
	/// # Errors
	///
	/// Will fail if any of the values are not strings.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let map = edb.list()?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn list(&self) -> EdbResult<HashMap<String, String>> {
		self.list_json()?
			.drain()
			.map(|(s, v)| match v.as_str() {
				Some(v_str) => Ok((s, v_str.to_string())),
				None => Err(format!("A value was not a string: key: {}, value: {}", s, v).into()),
			})
			.collect()
	}
	/// Returns a `HashMap<String, Json>` of all the data in this database.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// let map = edb.list()?;
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn list_json(&self) -> EdbResult<HashMap<String, Json>> {
		let mut s = Vec::new();
		self.list_writer(&mut s)?;
		Ok(serde_json::from_slice(&s)?)
	}
	/// Clears the database.
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// # let edb = EasyDB::new()?;
	/// edb.clear()?;
	/// # std::thread::sleep(std::time::Duration::from_secs(1));
	/// assert_eq!(edb.list()?.len(), 0);
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	pub fn clear(&self) -> EdbResult<()> {
		let map = self.list_json()?;
		self.clear_keys(map.keys().map(|k| &k[..]))?;
		Ok(())
	}
	fn clear_keys<'a, I>(&self, keys: I) -> EdbResult<()>
	where
		I: Iterator<Item = &'a str>,
	{
		for key in keys {
			self.delete(key)?;
		}
		Ok(())
	}
	/// An alternative to `get()` that works with a writer. Fetches data associated with `key` and
	/// writes into `value`, returning the status code.
	///
	/// The response should have the value that was originally set in JSON form. If the key was
	/// never set, was deleted, or has no data, the response will be an empty string: `""`.
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
	///
	/// # Example
	///
	/// ```
	/// # use crate::easydb::{EasyDB, errors::EdbError};
	/// let s = r#"
	/// UUID = "abcd"
	/// Token = "efgh"
	/// "#;
	/// let edb: EasyDB = s.parse()?;
	/// assert_eq!(edb.uuid(), "abcd");
	/// assert_eq!(edb.token(), "efgh");
	/// # Ok::<(), EdbError>(())
	/// ```
	///
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(toml::from_str(s)?)
	}
}
