use cert_util::{gen_ca_cert, gen_root_cert, gen_rsa_pkcs8_key_pem_and_file, gen_valid_date};
use chrono::Datelike;
use picky::key::PrivateKey;
use picky::x509::certificate::CertificateBuilder;
use picky::x509::csr::Attribute;
use picky::x509::date::UTCDate;
use picky::x509::extension::KeyUsage;
use picky::x509::key_id_gen_method::KeyIdGenMethod;
use picky::x509::name::{DirectoryName, GeneralName, NameAttr};
use picky::x509::{certificate::CertType, csr::Csr, Extension, Extensions};
use picky::{hash::HashAlgorithm, oids, signature::SignatureAlgorithm};
use std::error::Error;

// Generate a self-signed root certificate
fn main() -> Result<(), Box<dyn Error>> {
    let (root_key, _) =
        gen_rsa_pkcs8_key_pem_and_file("certs/root_pri.key", "certs/root_pub.key").unwrap();
    let (intermediate_pri, _) =
        gen_rsa_pkcs8_key_pem_and_file("certs/intermediate_pri.key", "certs/intermediate_pub.key")
            .unwrap();
    let (leaf_pri, _) =
        gen_rsa_pkcs8_key_pem_and_file("certs/leaf_pri.key", "certs/leaf_pub.key").unwrap();

    let (from_date, to_date) = gen_valid_date(3)?;
    let root = gen_root_cert("MyRootCa", from_date, to_date, &root_key, "certs/root.crt")?;
    assert_eq!(root.ty(), CertType::Root);
    let (from_date, to_date) = gen_valid_date(3)?;
    let intermediate = gen_ca_cert(
        "MyIntermediateCa",
        from_date,
        to_date,
        &root,
        &root_key,
        &intermediate_pri,
        "certs/intermediate.crt",
    )?;
    assert_eq!(intermediate.ty(), CertType::Intermediate);

    // Generate leaf certificate signed by intermediate authority

    let leaf_key = PrivateKey::from_pem_str(leaf_key_pem_str)?;
    let mut key_usage = KeyUsage::new(3);
    key_usage.set_digital_signature(false);
    key_usage.set_content_commitment(false);
    key_usage.set_key_encipherment(false);
    let extensions = Extensions(vec![
        // Extension::new_basic_constraints(None, None).into_non_critical(),
        // Extension::new_key_usage(key_usage).into_non_critical(),
        Extension::new_extended_key_usage(vec![
            oids::kp_client_auth(),
            oids::kp_server_auth(),
            oids::kp_code_signing(),
        ])
        .into_non_critical(),
        Extension::new_subject_alt_name(vec![
            GeneralName::new_dns_name("www.localhost.com")
                .unwrap()
                .into(),
            GeneralName::new_dns_name("localhost.com").unwrap().into(),
        ])
        .into_non_critical(),
    ]);

    let attr = Attribute::new_extension_request(extensions.0);

    println!("attr={:?}", attr);
    let mut my_name = DirectoryName::new_common_name("localhost");
    my_name.add_attr(NameAttr::StateOrProvinceName, "fujian");
    my_name.add_attr(NameAttr::CountryName, "China");
    // assert_eq!(
    //     my_name.to_string(),
    //     "CN=jmhuang,ST=fujian,C=China"
    // );
    let csr = Csr::generate_with_attributes(
        my_name,
        &leaf_key,
        SignatureAlgorithm::RsaPkcs1v15(HashAlgorithm::SHA2_256),
        vec![attr],
    )
    .unwrap();
    println!("{:?}", csr);

    let signed_leaf = CertificateBuilder::new()
        .validity(
            UTCDate::ymd(2020, 11, 1).unwrap(),
            UTCDate::ymd(2024, 1, 1).unwrap(),
        )
        .subject_from_csr(csr)
        .inherit_extensions_from_csr_attributes(true)
        .issuer_cert(&intermediate, &intermediate_key)
        .signature_hash_type(SignatureAlgorithm::RsaPkcs1v15(HashAlgorithm::SHA2_384))
        .key_id_gen_method(KeyIdGenMethod::SPKFullDER(HashAlgorithm::SHA2_512))
        .build()?;

    assert_eq!(signed_leaf.ty(), CertType::Leaf);

    // Check leaf using CA chain

    let chain = [intermediate, root];

    signed_leaf
        .verifier()
        .chain(chain.iter())
        .exact_date(&UTCDate::ymd(2020, 12, 20).unwrap())
        .verify()?;
    // If `not_after` date is behind…

    let err = signed_leaf
        .verifier()
        .chain(chain.iter())
        .exact_date(&UTCDate::ymd(2025, 1, 2).unwrap())
        .verify()
        .err()
        .unwrap();

    assert_eq!(
        err.to_string(),
        "invalid certificate \'CN=example,ST=fujian,C=China\': \
     certificate expired (not after: 2024-01-01 00:00:00, now: 2025-01-02 00:00:00)"
    );

    let root_pem = chain[1].to_pem()?;
    let intermediate_pem = chain[0].to_pem()?;
    let leaf_pem = signed_leaf.to_pem()?;
    std::fs::write("certs/root.crt", root_pem.data()).unwrap();
    std::fs::write("certs/intermediate.crt", intermediate_pem.data()).unwrap();
    std::fs::write("certs/leaf.crt", leaf_pem.data()).unwrap();
    Ok(())
}
