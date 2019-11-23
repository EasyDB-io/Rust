use error_chain::error_chain;

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