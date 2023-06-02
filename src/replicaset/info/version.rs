//! Detect the store version for Replica Set members.
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

use replisdk::agent::framework::NodeInfoFactoryArgs;
use replisdk::agent::framework::StoreVersionChain;
use replisdk::agent::framework::StoreVersionCommand;
use replisdk::agent::framework::StoreVersionCommandConf;
use replisdk::agent::framework::StoreVersionFile;
use replisdk::agent::models::StoreVersion;

static BUILD_INFO_EXTRACT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)Build Info: (\{.*\})")
        .expect("MongoDB build info regular expression failed to compile")
});

/// MongoD build information data returned parsed out of `mongod --version`.
#[derive(Debug, serde::Deserialize)]
struct MongoBuildInfo {
    #[serde(flatten)]
    extra: serde_json::Value,

    #[serde(rename = "gitVersion")]
    git_version: String,

    version: String,
}

/// Default command to detect the mongod version.
fn default_command_conf() -> StoreVersionCommandConf {
    StoreVersionCommandConf {
        args: vec!["--version".into()],
        command: "mongod".into(),
        env: Default::default(),
    }
}

/// Decode the output of mongod --version command into a [`StoreVersion`].
fn mongod_version_decode(data: Vec<u8>) -> Result<StoreVersion> {
    // Example output this function parses.
    // ```
    // db version v4.4.13
    // Build Info: {
    //     "version": "4.4.13",
    //     "gitVersion": "df25c71b8674a78e17468f48bcda5285decb9246",
    //     "openSSLVersion": "OpenSSL 1.1.1f  31 Mar 2020",
    //     "modules": [],
    //     "allocator": "tcmalloc",
    //     "environment": {
    //         "distmod": "ubuntu2004",
    //         "distarch": "x86_64",
    //         "target_arch": "x86_64"
    //     }
    // }
    // ```
    let data = String::from_utf8(data)?;
    let build_info = match BUILD_INFO_EXTRACT.captures(&data) {
        None => anyhow::bail!(VersionNotInOutput {}),
        Some(info) => info,
    };
    let build_info = build_info
        .get(1)
        .expect("regex matched but capture group not found")
        .as_str();
    let build_info: MongoBuildInfo = serde_json::from_str(build_info)?;
    let extra = if build_info.extra.is_null() {
        None
    } else {
        let extra = serde_json::to_string(&build_info.extra)?;
        Some(extra)
    };
    Ok(StoreVersion {
        checkout: Some(build_info.git_version),
        extra,
        number: build_info.version,
    })
}

/// Unable to find version information.
#[derive(Debug, thiserror::Error)]
#[error("unable to find version information")]
pub struct VersionNotInOutput {}

/// Configure the store version detection strategies.
pub fn configure_strategies(
    args: NodeInfoFactoryArgs<'_, crate::conf::Conf>,
) -> Result<StoreVersionChain> {
    let chain = StoreVersionChain::default();

    // Try checking the mongod command first.
    let conf = args
        .conf
        .custom
        .version_detect
        .command
        .clone()
        .unwrap_or_else(default_command_conf);
    let strategy = StoreVersionCommand::with_conf(conf)
        .decode(mongod_version_decode)
        .finish();
    let chain = chain.strategy(strategy);

    // Try checking a version file after.
    let mut chain = chain;
    if let Some(ref path) = args.conf.custom.version_detect.file {
        let strategy = StoreVersionFile::new(path).decode(mongod_version_decode);
        chain = chain.strategy(strategy);
    }

    Ok(chain)
}

#[cfg(test)]
mod tests {
    use super::mongod_version_decode;

    const BUILD_INFO: &str = r#"db version v4.4.13
    Build Info: {
        "version": "4.4.13",
        "gitVersion": "df25c71b8674a78e17468f48bcda5285decb9246",
        "openSSLVersion": "OpenSSL 1.1.1f  31 Mar 2020",
        "modules": [],
        "allocator": "tcmalloc",
        "environment": {
            "distmod": "ubuntu2004",
            "distarch": "x86_64",
            "target_arch": "x86_64"
        }
    }"#;

    #[test]
    fn build_info_extracted() {
        let data = Vec::from(BUILD_INFO);
        let version = mongod_version_decode(data).unwrap();
        assert_eq!(
            version.checkout,
            Some("df25c71b8674a78e17468f48bcda5285decb9246".into())
        );
        assert_eq!(version.number, "4.4.13".to_string());
        assert_eq!(
            version.extra,
            Some(
                r#"{"openSSLVersion":"OpenSSL 1.1.1f  31 Mar 2020","modules":[],"allocator":"tcmalloc","environment":{"distmod":"ubuntu2004","distarch":"x86_64","target_arch":"x86_64"}}"#.into()
            )
        )
    }

    #[test]
    fn build_info_not_found() {
        let data = Vec::from("not build info {}");
        match mongod_version_decode(data) {
            Ok(version) => panic!("expected error, got version {:?}", version),
            Err(error) if error.is::<super::VersionNotInOutput>() => (),
            Err(error) => panic!("expected VersionNotInOutput error, got error {:?}", error),
        }
    }
}
