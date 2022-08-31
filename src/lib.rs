//!
//! # Overview
//!
//! The osv client library provides a thin layer of abstraction
//! over the open source vulnerability (osv) database API. The osv database is
//! an open, precise and distributed approach to producing and consuming
//! vulnerability information for open source projects.
//!
//! This library currently provides a mean to find out whether a particular package
//! version is affected by any vulnerabilities and to fetch specific information
//! about a vulnerability within a number of different package ecosystems. It
//! is the intention to follow along with osv evolution and provide quality
//! type safe bindings to API for rust clients.
//!
//! The models and endpoints are derived from the documentation
//! published on <https://osv.dev/> directly.
//!
//!
//! # Examples
//!
//! ```
//! use comfy_table::Table;
//! use osv::Ecosystem::PyPI;
//! use textwrap::termwidth;
//!
//! #[async_std::main]
//! async fn main() -> Result<(), osv::ApiError> {
//!
//!    if let Some(vulns) = osv::query_package("jinja2", "2.4.1", PyPI).await? {
//!        let default = String::from("-");
//!        let linewrap = (termwidth() as f32 / 3.0 * 2.0).round() as usize;
//!        let mut table = Table::new();
//!        table.set_header(vec!["Vulnerability ID", "Details"]);
//!        for vuln in &vulns {
//!            let details = vuln.details.as_ref().unwrap_or(&default);
//!            let details = textwrap::wrap(details, linewrap).join("\n");
//!            table.add_row(vec![&vuln.id, &details]);
//!        }
//!        println!("{table}");
//!    }
//!    Ok(())
//!}
//! ```
//!
//! There are more examples [here](https://github.com/gcmurphy/osv/tree/master/examples) that demonstrate usage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surf::http::StatusCode;
use thiserror::Error;
use url::Url;

/// Package identifies the code library or command that
/// is potentially affected by a particular vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    /// The name of the package or dependency.
    pub name: String,

    /// The ecosystem identifies the overall library ecosystem that this
    /// package can be obtained from.
    pub ecosystem: Ecosystem,

    /// The purl field is a string following the [Package URL
    /// specification](https://github.com/package-url/purl-spec) that identifies the
    /// package. This field is optional but recommended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
}

/// A commit is a full SHA1 Git hash in hex format.
pub type Commit = String;

/// Version is arbitrary string representing the version of a package.
pub type Version = String;

/// The package ecosystem that the vulnerabilities in the OSV database
/// are associated with.
#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Ecosystem {
    Go,
    #[serde(rename = "npm")]
    Npm,
    #[serde(rename = "OSS-Fuzz")]
    OssFuzz,
    PyPI,
    RubyGems,
    #[serde(rename = "crates.io")]
    CratesIO,
    Packagist,
    Maven,
    NuGet,
    Linux,
    Debian,
    Hex,
    Android,
    #[serde(rename = "GitHub Actions")]
    GitHubActions,
    Pub,
}

/// Type of the affected range supplied. This can be an ecosystem
/// specific value, semver, or a git commit hash.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RangeType {
    /// Default for the case where a range type is omitted.
    Unspecified,

    /// The versions introduced and fixed are full-length Git commit hashes.
    Git,

    /// The versions introduced and fixed are semantic versions as defined by SemVer 2.0.0.
    Semver,

    /// The versions introduced and fixed are arbitrary, uninterpreted strings specific to the
    /// package ecosystem
    Ecosystem,
}

/// The event captures information about the how and when
/// the package was affected by the vulnerability.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Event {
    /// The version or commit in which the vulnerability was
    /// introduced.
    Introduced(String),

    /// The version which the vulnerability was fixed.
    Fixed(String),

    /// The upper limit on the range being described.
    Limit(String),
}

/// The range of versions of a package for which
/// it is affected by the vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub struct Range {
    /// The format that the range events are specified in, for
    /// example SEMVER or GIT.
    #[serde(rename = "type")]
    pub range_type: RangeType,

    /// The ranges object’s repo field is the URL of the package’s code repository. The value
    /// should be in a format that’s directly usable as an argument for the version control
    /// system’s clone command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,

    /// Represent a status timeline for how the vulnerability affected the package. For
    /// example when the vulnerability was first introduced into the codebase.
    pub events: Vec<Event>,
}

/// The versions of the package that are affected
/// by a particular vulnerability. The affected ranges can include
/// when the vulnerability was first introduced and also when it
/// was fixed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Affected {
    /// The package that is affected by the vulnerability
    pub package: Package,

    /// The range of versions or git commits that this vulnerability
    /// was first introduced and/or version that it was fixed in.
    pub ranges: Vec<Range>,

    /// Each string is a single affected version in whatever version syntax is
    /// used by the given package ecosystem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<String>>,

    /// A JSON object that holds any additional information about the
    /// vulnerability as defined by the ecosystem for which the record applies.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecosystem_specific: Option<serde_json::Value>,

    /// A JSON object to hold any additional information about the range
    /// from which this record was obtained. The meaning of the values within
    /// the object is entirely defined by the database.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_specific: Option<serde_json::Value>,
}

/// The type of reference information that has been provided. Examples include
/// links to the original report, external advisories, or information about the
/// fix.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReferenceType {
    #[serde(rename = "NONE")]
    Undefined,
    Web,
    Advisory,
    Report,
    Fix,
    Package,
    Article,
}

/// Reference to additional information about the vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub struct Reference {
    /// The type of reference this URL points to.
    #[serde(rename = "type")]
    pub reference_type: ReferenceType,

    /// The url where more information can be obtained about
    /// the vulnerability or associated the fix.
    pub url: String,
}

/// The [`SeverityType`](SeverityType) describes the quantitative scoring method used to rate the
/// severity of the vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub enum SeverityType {
    /// The severity score was arrived at by using an unspecified
    /// scoring method.
    #[serde(rename = "UNSPECIFIED")]
    Unspecified,

    /// A CVSS vector string representing the unique characteristics and severity of the
    /// vulnerability using a version of the Common Vulnerability Scoring System notation that is
    /// >= 3.0 and < 4.0 (e.g.`"CVSS:3.1/AV:N/AC:H/PR:N/UI:N/S:C/C:H/I:N/A:N"`).
    #[serde(rename = "CVSS_V3")]
    CVSSv3,
}

/// The type and score used to describe the severity of a vulnerability using one
/// or more quantitative scoring methods.
#[derive(Debug, Serialize, Deserialize)]
pub struct Severity {
    /// The severity type property must be a [`SeverityType`](SeverityType), which describes the
    /// quantitative method used to calculate the associated score.
    #[serde(rename = "type")]
    pub severity_type: SeverityType,

    /// The score property is a string representing the severity score based on the
    /// selected severity type.
    pub score: String,
}

/// Provides a way to give credit for the discovery, confirmation, patch or other events in the
/// life cycle of a vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub struct Credit {
    pub name: String,
    pub contact: Vec<String>,
}

/// A vulnerability is the standard exchange format that is
/// defined by the OSV schema <https://ossf.github.io/osv-schema/>.
///
/// This is the entity that is returned when vulnerable data exists for
/// a given package or when requesting information about a specific vulnerability
/// by unique identifier.
#[derive(Debug, Serialize, Deserialize)]
pub struct Vulnerability {
    /// The schema_version field is used to indicate which version of the OSV schema a particular
    /// vulnerability was exported with.
    pub schema_version: String,
    /// The id field is a unique identifier for the vulnerability entry. It is a string of the
    /// format <DB>-<ENTRYID>, where DB names the database and ENTRYID is in the format used by the
    /// database. For example: “OSV-2020-111”, “CVE-2021-3114”, or “GHSA-vp9c-fpxx-744v”.
    pub id: String,

    /// The published field gives the time the entry should be considered to have been published,
    /// as an RFC3339-formatted time stamp in UTC (ending in “Z”).
    pub published: DateTime<Utc>,

    /// The modified field gives the time the entry was last modified, as an RFC3339-formatted
    /// timestamptime stamp in UTC (ending in “Z”).
    pub modified: DateTime<Utc>,

    /// The withdrawn field gives the time the entry should be considered to have been withdrawn,
    /// as an RFC3339-formatted timestamp in UTC (ending in “Z”). If the field is missing, then the
    /// entry has not been withdrawn. Any rationale for why the vulnerability has been withdrawn
    /// should go into the summary text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<DateTime<Utc>>,

    /// The aliases field gives a list of IDs of the same vulnerability in other databases, in the
    /// form of the id field. This allows one database to claim that its own entry describes the
    /// same vulnerability as one or more entries in other databases. Or if one database entry has
    /// been deduplicated into another in the same database, the duplicate entry could be written
    /// using only the id, modified, and aliases field, to point to the canonical one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,

    /// The related field gives a list of IDs of closely related vulnerabilities, such as the same
    /// problem in alternate ecosystems.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<String>>,

    /// The summary field gives a one-line, English textual summary of the vulnerability. It is
    /// recommended that this field be kept short, on the order of no more than 120 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// The details field gives additional English textual details about the vulnerability. The
    /// details field is CommonMark markdown (a subset of GitHub-Flavored Markdown). Display code
    /// may at its discretion sanitize the input further, such as stripping raw HTML and links that
    /// do not start with http:// or https://. Databases are encouraged not to include those in the
    /// first place. (The goal is to balance flexibility of presentation with not exposing
    /// vulnerability database display sites to unnecessary vulnerabilities.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Indicates the specific package ranges that are affected by this vulnerability.
    pub affected: Vec<Affected>,

    /// An optional list of external reference's that provide more context about this
    /// vulnerability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,

    /// The severity field is a JSON array that allows generating systems to describe the severity
    /// of a vulnerability using one or more quantitative scoring methods. Each severity item is a
    /// object specifying a type and score property.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<Vec<Severity>>,

    /// Provides a way to give credit for the discovery, confirmation, patch or other events in the
    /// life cycle of a vulnerability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits: Option<Vec<Credit>>,

    /// Top level field to hold any additional information about the vulnerability as defined
    /// by the database from which the record was obtained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_specific: Option<serde_json::Value>,
}

/// A Request encapsulates the different payloads that will be accepted by the
/// osv.dev API server. You can either submit a query to the server using a
/// commit hash or alternatively a package and version pair.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Request {
    /// Query the vulnerability sources by commit ID
    CommitQuery { commit: Commit },

    /// Query the vulnerability sources by package and version pair.
    PackageQuery { version: Version, package: Package },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Response {
    Vulnerabilities { vulns: Vec<Vulnerability> },
    NoResult(serde_json::Value),
}

/// ApiError is the common error type when a request is rejected
/// by the api.osv.dev endpoint, the response is not understood
/// by the client or there is an underlying connection
/// problem.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ApiError {
    #[error("requested resource {0} not found")]
    NotFound(String),

    #[error("invalid request url: {0:?}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("serialization failure: {0:?}")]
    SerializationError(#[from] serde_json::Error),

    #[error("request to osv endpoint failed: {0:?}")]
    RequestFailed(surf::Error),

    #[error("unexpected error has occurred")]
    Unexpected,
}

impl From<surf::Error> for ApiError {
    fn from(err: surf::Error) -> Self {
        ApiError::RequestFailed(err)
    }
}

///
/// Query the underlying Open Source Vulnerability (osv) database for
/// any vulnerabilities associated with either a package or a commit.
///
/// The request can either be based on a commit or package and version
/// tuple. When querying a package you also need to specify the package
/// ecosystem the package belongs to.
///
/// Note that - [`query_commit`](query_commit) and [`query_package`](query_package) are convenience wrappers
/// for this function and should be favoured over calling [`query`](query) directly.
///
///
/// # Examples
///
/// ```
/// # use async_std::task;
/// # task::block_on(async {
/// let ver = osv::Version::from("2.4.1");
/// let pkg = "jinja2".to_string();
/// let req = osv::Request::PackageQuery {
///             version: ver,
///             package: osv::Package {
///                name: pkg,
///                ecosystem: osv::Ecosystem::PyPI,
///                purl: None,
///            }
///     };
///
/// let resp = osv::query(&req).await.expect("vulnerabilities expected");
/// println!("{:#?}", resp.unwrap());
/// # });
/// ```
///
///
pub async fn query(q: &Request) -> Result<Option<Vec<Vulnerability>>, ApiError> {
    let mut res = surf::post("https://api.osv.dev/v1/query")
        .body_json(q)?
        .await?;

    match res.status() {
        StatusCode::NotFound => {
            let err = match q {
                Request::PackageQuery {
                    version: _,
                    package: pkg,
                } => {
                    format!("package - `{}`", pkg.name)
                }
                Request::CommitQuery { commit: c } => {
                    format!("commit - `{}`", c)
                }
            };
            Err(ApiError::NotFound(err))
        }
        _ => {
            let vulns: Response = res.body_json().await?;
            match vulns {
                Response::Vulnerabilities { vulns: vs } => Ok(Some(vs)),
                _ => Ok(None),
            }
        }
    }
}

///
/// Query the Open Source Vulnerability (osv) database for
/// vulnerabilities associated with the specified package
/// and version.
///
/// See <https://osv.dev/docs/#operation/OSV_QueryAffected> for more
/// details.
///
/// # Examples
///
/// ```
/// use osv::query_package;
/// use osv::Ecosystem::PyPI;
/// # use async_std::task;
/// # task::block_on(async {
///     let pkg = "jinja2";
///     let ver = "2.4.1";
///     if let Some(vulns) = query_package(pkg, ver, PyPI).await.unwrap() {
///         for vuln in &vulns {
///             println!("{:#?} - {:#?}", vuln.id, vuln.details);
///             for affected in &vuln.affected {
///                 println!("    {:#?}", affected.ranges);
///             }
///         }
///     } else {
///         println!("no known vulnerabilities for {} v{}", pkg, ver);
///     }
/// # });
/// ```
pub async fn query_package(
    name: &str,
    version: &str,
    ecosystem: Ecosystem,
) -> Result<Option<Vec<Vulnerability>>, ApiError> {
    let req = Request::PackageQuery {
        version: Version::from(version),
        package: Package {
            name: name.to_string(),
            ecosystem,
            purl: None,
        },
    };

    query(&req).await
}

///
/// Query the Open Source Vulnerability (osv) database for
/// vulnerabilities based on a Git commit SHA1.
///
/// See <https://osv.dev/docs/#operation/OSV_QueryAffected> for more details
/// and examples.
///
/// # Examples
///
/// ```
/// # use async_std::task;
/// # use osv::query_commit;
/// # task::block_on(async {
/// let vulnerable = query_commit("6879efc2c1596d11a6a6ad296f80063b558d5e0f")
///         .await
///         .expect("api error");
///
/// match vulnerable {
///     Some(affected) => println!("{:#?}", affected),
///     None => println!("all clear!"),
/// }
/// # });
/// ```
///
pub async fn query_commit(commit: &str) -> Result<Option<Vec<Vulnerability>>, ApiError> {
    let req = Request::CommitQuery {
        commit: Commit::from(commit),
    };
    query(&req).await
}

///
/// Query the osv database for vulnerability by ID.
///
/// # Examples
///
/// ```
/// # use async_std::task;
/// use osv::vulnerability;
/// # task::block_on(async {
/// let vuln = vulnerability("OSV-2020-484").await.unwrap();
/// assert!(vuln.id.eq("OSV-2020-484"));
///
/// # });
/// ```
pub async fn vulnerability(vuln_id: &str) -> Result<Vulnerability, ApiError> {
    let base = Url::parse("https://api.osv.dev/v1/vulns/")?;
    let req = base.join(vuln_id)?;
    let mut res = surf::get(req.as_str()).await?;
    if res.status() == StatusCode::NotFound {
        Err(ApiError::NotFound(vuln_id.to_string()))
    } else {
        let vuln: Vulnerability = res.body_json().await?;
        Ok(vuln)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_package_query() {
        let req = Request::PackageQuery {
            version: Version::from("2.4.1"),
            package: Package {
                name: "jinja2".to_string(),
                ecosystem: Ecosystem::PyPI,
                purl: None,
            },
        };
        let res = query(&req).await.unwrap();
        assert!(res.is_some());
    }

    #[async_std::test]
    async fn test_package_query_wrapper() {
        let res = query_package("jinja2", "2.4.1", Ecosystem::PyPI)
            .await
            .unwrap();
        assert!(res.is_some());
    }

    #[async_std::test]
    async fn test_invalid_packagename() {
        let res = query_package(
            "asdfasdlfkjlksdjfklsdjfklsdjfklds",
            "0.0.1",
            Ecosystem::PyPI,
        )
        .await
        .unwrap();
        assert!(res.is_none());
    }

    #[async_std::test]
    async fn test_commit_query() {
        let req = Request::CommitQuery {
            commit: Commit::from("6879efc2c1596d11a6a6ad296f80063b558d5e0f"),
        };
        let res = query(&req).await.unwrap();
        assert!(res.is_some());
    }

    #[async_std::test]
    async fn test_commit_query_wrapper() {
        let res = query_commit("6879efc2c1596d11a6a6ad296f80063b558d5e0f")
            .await
            .unwrap();
        assert!(res.is_some());
    }

    #[async_std::test]
    async fn test_invalid_commit() {
        let res = query_commit("zzzz").await.unwrap();
        assert!(res.is_none());
    }

    #[async_std::test]
    async fn test_vulnerability() {
        let res = vulnerability("OSV-2020-484").await;
        assert!(res.is_ok());
    }

    #[async_std::test]
    async fn test_get_missing_cve() {
        let res = vulnerability("CVE-2014-0160").await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_no_serialize_null_fields() {
        let vuln = Vulnerability {
          schema_version: "1.3.0".to_string(),
          id: "OSV-2020-484".to_string(),
          published: chrono::Utc::now(),
          modified: chrono::Utc::now(),
          withdrawn: None,
          aliases: None,
          related: None,
          summary: None,
          details: None,
          affected: vec![],
          references: None,
          severity: None,
          credits: None,
          database_specific: None
        };

        let as_json = serde_json::json!(vuln);
        let str_json = as_json.to_string();
        assert!(!str_json.contains("withdrawn"));
        assert!(!str_json.contains("aliases"));
        assert!(!str_json.contains("related"));
        assert!(!str_json.contains("summary"));
        assert!(!str_json.contains("details"));
        assert!(!str_json.contains("references"));
        assert!(!str_json.contains("severity"));
        assert!(!str_json.contains("credits"));
        assert!(!str_json.contains("database_specific"));
    }


}
