//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2026 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//


//! Shared helpers for the integration tests.
//!
//! The tests drive the real `ralfpack` binary and verify the results with the `unwgt` tool from
//! <https://github.com/rdkcentral/ralf-utils>.  `unwgt` is found via (in order) the `UNWGT` env var, the
//! `RALF_UTILS_DIR` env var (a directory containing `unwgt`), or `PATH`.  If it can't be found the tests
//! are skipped locally, but fail when the `CI` env var is set.

#![allow(dead_code)]

use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::hash::MessageDigest;
use openssl::pkcs12::Pkcs12;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{BasicConstraints, KeyUsage};
use openssl::x509::{X509, X509NameBuilder};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

pub const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/package_config_v1.1.0.json");

pub struct Output2 {
    pub success: bool,
    pub text: String,
}

fn run(cmd: &mut Command) -> Output2 {
    let out: Output = cmd.output().unwrap_or_else(|e| panic!("failed to run {cmd:?}: {e}"));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Output2 {
        success: out.status.success(),
        text,
    }
}

/// Runs `ralfpack` with the given args, returning its result.
pub fn ralfpack<I, S>(args: I) -> Output2
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let bin = std::env::var_os("RALFPACK_BIN").unwrap_or_else(|| env!("CARGO_BIN_EXE_ralfpack").into());
    run(Command::new(bin).args(args))
}

/// Runs `ralfpack` and panics (with its output) if it fails.
pub fn ralfpack_ok<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = ralfpack(args);
    assert!(out.success, "ralfpack failed:\n{}", out.text);
    out.text
}

/// Locates `unwgt`, or `None` if it isn't available.
pub fn find_unwgt() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("UNWGT") {
        return Some(p.into());
    }
    if let Some(dir) = std::env::var_os("RALF_UTILS_DIR") {
        let p = Path::new(&dir).join("unwgt");
        return p.is_file().then_some(p);
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join("unwgt"))
        .find(|p| p.is_file())
}

/// Returns a configured test environment, or `None` if the test should be skipped.
pub fn setup() -> Option<Env> {
    match find_unwgt() {
        Some(unwgt) => Some(Env::new(unwgt)),
        None if std::env::var_os("CI").is_some() => {
            panic!("unwgt not found; set UNWGT or RALF_UTILS_DIR (required when CI is set)")
        }
        None => {
            eprintln!("SKIPPED: unwgt not found (set UNWGT or RALF_UTILS_DIR)");
            None
        }
    }
}

pub struct Creds {
    pub ca_pem: PathBuf,
    pub cert_pem: PathBuf,
    pub key_pem: PathBuf,
    pub p12: PathBuf,
}

pub struct Env {
    pub dir: TempDir,
    pub unwgt: PathBuf,
    pub content: PathBuf,
    /// Credentials signed by `ca`.
    pub creds: Creds,
    /// A second, unrelated CA; signatures must not verify against it.
    pub other_ca_pem: PathBuf,
}

impl Env {
    fn new(unwgt: PathBuf) -> Env {
        let dir = TempDir::new().unwrap();

        let content = dir.path().join("content");
        fs::create_dir_all(content.join("web")).unwrap();
        fs::write(content.join("web/index.html"), "<html><body>hello</body></html>\n").unwrap();
        fs::write(
            content.join("data.bin"),
            (0..=255u8).cycle().take(100_000).collect::<Vec<u8>>(),
        )
        .unwrap();

        let (ca_cert, ca_key) = make_cert("Ralfpack Test CA", None);
        let (leaf_cert, leaf_key) = make_cert("Ralfpack Test Signer", Some((&ca_cert, &ca_key)));
        let creds = Creds {
            ca_pem: write(&dir, "ca.pem", &ca_cert.to_pem().unwrap()),
            cert_pem: write(&dir, "signer.pem", &leaf_cert.to_pem().unwrap()),
            key_pem: write(&dir, "signer.key", &leaf_key.private_key_to_pem_pkcs8().unwrap()),
            p12: {
                let mut chain = openssl::stack::Stack::new().unwrap();
                chain.push(ca_cert).unwrap();
                let p12 = Pkcs12::builder()
                    .name("signer")
                    .pkey(&leaf_key)
                    .cert(&leaf_cert)
                    .ca(chain)
                    .build2("")
                    .unwrap();
                write(&dir, "signer.p12", &p12.to_der().unwrap())
            },
        };

        let (other_ca, _) = make_cert("Some Other CA", None);
        let other_ca_pem = write(&dir, "other-ca.pem", &other_ca.to_pem().unwrap());

        Env {
            dir,
            unwgt,
            content,
            creds,
            other_ca_pem,
        }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Creates an unsigned package with extra `create` args.
    pub fn create_unsigned(&self, name: &str, extra: &[&str]) -> PathBuf {
        let pkg = self.path(name);
        ralfpack_ok(
            [
                "create",
                "--content",
                self.content.to_str().unwrap(),
                "--config",
                CONFIG,
            ]
            .into_iter()
            .chain(extra.iter().copied())
            .chain([pkg.to_str().unwrap()]),
        );
        pkg
    }

    /// Creates a package signed with the PEM key, certificate and CA chain.
    pub fn create_signed(&self, name: &str, extra: &[&str]) -> PathBuf {
        let c = &self.creds;
        let mut args = vec![
            "--key",
            c.key_pem.to_str().unwrap(),
            "--certificate",
            c.cert_pem.to_str().unwrap(),
            "--certificate-chain",
            c.ca_pem.to_str().unwrap(),
        ];
        args.extend(extra);
        self.create_unsigned(name, &args)
    }

    /// Creates a package signed by a certificate (issued by the trusted CA) that expired a day ago.
    /// Returns the package and the path of the CA to verify it with.
    pub fn create_signed_with_expired_cert(&self, name: &str) -> (PathBuf, PathBuf) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let day = 86_400;

        let (ca_cert, ca_key) = make_cert("Expiry Test CA", None);
        let (cert, key) = make_cert_valid(
            "Expired Signer",
            Some((&ca_cert, &ca_key)),
            Asn1Time::from_unix(now - 30 * day).unwrap(),
            Asn1Time::from_unix(now - day).unwrap(),
        );
        let ca_pem = write(&self.dir, "expiry-ca.pem", &ca_cert.to_pem().unwrap());
        let cert_pem = write(&self.dir, "expired.pem", &cert.to_pem().unwrap());
        let key_pem = write(&self.dir, "expired.key", &key.private_key_to_pem_pkcs8().unwrap());

        let pkg = self.create_unsigned(
            name,
            &[
                "--key",
                key_pem.to_str().unwrap(),
                "--certificate",
                cert_pem.to_str().unwrap(),
                "--certificate-chain",
                ca_pem.to_str().unwrap(),
                "--skip-certificate-expiry-check",
            ],
        );
        (pkg, ca_pem)
    }

    /// Runs `unwgt` with the given args.
    pub fn unwgt<I, S>(&self, args: I) -> Output2
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        run(Command::new(&self.unwgt).args(args))
    }

    /// `unwgt --verify` against the given trusted CA.
    pub fn unwgt_verify(&self, pkg: &Path, ca: &Path) -> Output2 {
        self.unwgt(["--verify", "-f", pkg.to_str().unwrap(), "-c", ca.to_str().unwrap()])
    }

    /// Extracts the package content with `unwgt` (no verification) and returns the directory.
    pub fn unwgt_extract(&self, pkg: &Path, out: &str) -> PathBuf {
        let dest = self.path(out);
        fs::create_dir_all(&dest).unwrap();
        let r = self.unwgt(["-x", "-n", "-f", pkg.to_str().unwrap(), "-C", dest.to_str().unwrap()]);
        assert!(r.success, "unwgt extract failed:\n{}", r.text);
        dest
    }
}

fn write(dir: &TempDir, name: &str, data: &[u8]) -> PathBuf {
    let p = dir.path().join(name);
    fs::write(&p, data).unwrap();
    p
}

/// Creates a self-signed CA (`issuer == None`) or a leaf certificate issued by the given CA.
fn make_cert(cn: &str, issuer: Option<(&X509, &PKey<Private>)>) -> (X509, PKey<Private>) {
    // The signing tools require at least 3 years of validity remaining.
    make_cert_valid(
        cn,
        issuer,
        Asn1Time::days_from_now(0).unwrap(),
        Asn1Time::days_from_now(3650).unwrap(),
    )
}

fn make_cert_valid(
    cn: &str,
    issuer: Option<(&X509, &PKey<Private>)>,
    not_before: Asn1Time,
    not_after: Asn1Time,
) -> (X509, PKey<Private>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();

    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", cn).unwrap();
    let name = name.build();

    let mut b = X509::builder().unwrap();
    b.set_version(2).unwrap();
    let mut serial = BigNum::new().unwrap();
    serial.rand(127, MsbOption::MAYBE_ZERO, false).unwrap();
    b.set_serial_number(&serial.to_asn1_integer().unwrap()).unwrap();
    b.set_subject_name(&name).unwrap();
    b.set_pubkey(&key).unwrap();
    b.set_not_before(&not_before).unwrap();
    b.set_not_after(&not_after).unwrap();

    let is_ca = issuer.is_none();
    if is_ca {
        b.append_extension(BasicConstraints::new().critical().ca().build().unwrap())
            .unwrap();
        b.append_extension(KeyUsage::new().critical().key_cert_sign().crl_sign().build().unwrap())
            .unwrap();
        b.set_issuer_name(&name).unwrap();
    } else {
        b.append_extension(BasicConstraints::new().critical().build().unwrap())
            .unwrap();
        b.append_extension(KeyUsage::new().critical().digital_signature().build().unwrap())
            .unwrap();
        b.set_issuer_name(issuer.unwrap().0.subject_name()).unwrap();
    }

    b.sign(issuer.map_or(&key, |i| i.1), MessageDigest::sha256()).unwrap();
    (b.build(), key)
}

/// Returns the contents of every regular file under `dir`, keyed by relative path.
pub fn read_tree(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            (rel, fs::read(e.path()).unwrap())
        })
        .collect()
}

/// Flips a byte in the middle of the largest entry (the content layer) of the zip based package.
pub fn tamper_largest_blob(pkg: &Path) {
    use std::io::{Read, Write};

    let mut zin = zip::ZipArchive::new(fs::File::open(pkg).unwrap()).unwrap();
    let mut entries = Vec::new();
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        entries.push((f.name().to_string(), data));
    }
    drop(zin);

    let largest = (0..entries.len()).max_by_key(|&i| entries[i].1.len()).unwrap();
    let mid = entries[largest].1.len() / 2;
    entries[largest].1[mid] ^= 0xff;

    let mut zout = zip::ZipWriter::new(fs::File::create(pkg).unwrap());
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        zout.start_file(name, opts).unwrap();
        zout.write_all(&data).unwrap();
    }
    zout.finish().unwrap();
}
