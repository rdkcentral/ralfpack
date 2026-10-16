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

//! Creates signed and unsigned packages with `ralfpack` and checks them with `unwgt` from ralf-utils.

mod common;

use common::*;

macro_rules! env_or_skip {
    () => {
        match setup() {
            Some(env) => env,
            None => return,
        }
    };
}

#[test]
fn unsigned_package_is_readable_but_fails_verification() {
    let env = env_or_skip!();
    let pkg = env.create_unsigned("unsigned.ralf", &[]);

    let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
    assert!(!v.success, "unsigned package must not verify:\n{}", v.text);

    let extracted = env.unwgt_extract(&pkg, "unsigned-out");
    assert_eq!(read_tree(&extracted), read_tree(&env.content));
}

#[test]
fn signed_package_verifies() {
    let env = env_or_skip!();
    let pkg = env.create_signed("signed.ralf", &[]);

    let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
    assert!(v.success, "signed package failed verification:\n{}", v.text);

    let s = env.unwgt([
        "-S",
        "-f",
        pkg.to_str().unwrap(),
        "-c",
        env.creds.ca_pem.to_str().unwrap(),
    ]);
    assert!(s.success, "{}", s.text);
    assert!(s.text.contains("Ralfpack Test Signer"), "{}", s.text);
}

#[test]
fn signed_package_content_and_metadata_roundtrip() {
    let env = env_or_skip!();
    let pkg = env.create_signed("roundtrip.ralf", &[]);

    let extracted = env.unwgt_extract(&pkg, "roundtrip-out");
    assert_eq!(read_tree(&extracted), read_tree(&env.content));

    let m = env.unwgt([
        "-M",
        "-f",
        pkg.to_str().unwrap(),
        "-c",
        env.creds.ca_pem.to_str().unwrap(),
    ]);
    assert!(m.success, "{}", m.text);
    assert!(m.text.contains("com.sky.myapp"), "{}", m.text);
}

#[test]
fn signed_with_pkcs12_verifies() {
    let env = env_or_skip!();
    let pkg = env.create_unsigned("p12.ralf", &["--pkcs12", env.creds.p12.to_str().unwrap()]);

    let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
    assert!(v.success, "{}", v.text);
}

#[test]
fn signing_an_unsigned_package_afterwards_verifies() {
    let env = env_or_skip!();
    let pkg = env.create_unsigned("late-sign.ralf", &[]);
    assert!(!env.unwgt_verify(&pkg, &env.creds.ca_pem).success);

    ralfpack_ok([
        "sign",
        "--pkcs12",
        env.creds.p12.to_str().unwrap(),
        pkg.to_str().unwrap(),
    ]);
    let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
    assert!(v.success, "{}", v.text);
}

#[test]
fn verification_fails_with_untrusted_ca() {
    let env = env_or_skip!();
    let pkg = env.create_signed("untrusted.ralf", &[]);

    let v = env.unwgt_verify(&pkg, &env.other_ca_pem);
    assert!(
        !v.success,
        "verification against an unrelated CA must fail:\n{}",
        v.text
    );
}

#[test]
fn tampered_package_fails_verification() {
    let env = env_or_skip!();
    let pkg = env.create_signed("tampered.ralf", &["--image-format", "tar"]);
    assert!(env.unwgt_verify(&pkg, &env.creds.ca_pem).success);

    tamper_largest_blob(&pkg);

    let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
    assert!(!v.success, "tampered package must not verify:\n{}", v.text);
}

#[test]
fn all_image_formats_create_verifiable_packages() {
    let env = env_or_skip!();
    for format in [
        "tar",
        "tar.gz",
        "tar.zstd",
        "erofs",
        "erofs.lz4",
        "erofs.zstd",
        "erofs.nocmpr",
    ] {
        let name = format!("fmt-{format}.ralf");
        let pkg = env.create_signed(&name, &["--image-format", format]);

        let v = env.unwgt_verify(&pkg, &env.creds.ca_pem);
        assert!(v.success, "{format}: verification failed:\n{}", v.text);

        // libralf can't read zstd compressed EROFS images yet, so only verify those
        if format == "erofs.zstd" {
            continue;
        }

        let out = env.unwgt_extract(&pkg, &format!("fmt-{format}-out"));
        assert_eq!(read_tree(&out), read_tree(&env.content), "{format}: content mismatch");
    }
}

#[test]
fn ralfpack_verify_accepts_its_own_signed_package() {
    let env = env_or_skip!();
    let pkg = env.create_signed("self-verify.ralf", &[]);

    let ok = ralfpack([
        "verify",
        "--ca-roots",
        env.creds.ca_pem.to_str().unwrap(),
        pkg.to_str().unwrap(),
    ]);
    assert!(ok.success, "{}", ok.text);

    let bad = ralfpack([
        "verify",
        "--ca-roots",
        env.other_ca_pem.to_str().unwrap(),
        pkg.to_str().unwrap(),
    ]);
    assert!(!bad.success, "{}", bad.text);
}

#[test]
fn expired_certificate_is_only_rejected_when_expiry_check_enabled() {
    let env = env_or_skip!();
    let (pkg, ca) = env.create_signed_with_expired_cert("expired.ralf");
    let (pkg, ca) = (pkg.to_str().unwrap(), ca.to_str().unwrap());

    // unwgt doesn't check certificate expiry by default
    let v = env.unwgt(["--verify", "-f", pkg, "-c", ca]);
    assert!(
        v.success,
        "expired cert should verify without the expiry check:\n{}",
        v.text
    );

    let v = env.unwgt(["--verify", "--enable-cert-expiry-check", "-f", pkg, "-c", ca]);
    assert!(!v.success, "expired cert must fail with the expiry check:\n{}", v.text);
}
