use cert_util::{
    gen_ca_cert, gen_cert_by_ca, gen_root_cert, gen_rsa_key_pem_and_file, gen_valid_date,
};
use picky::x509::csr::Attribute;
use picky::x509::date::UTCDate;
use picky::x509::extension::KeyUsage;
use picky::x509::name::{DirectoryName, GeneralName, NameAttr};
use picky::x509::{certificate::CertType, csr::Csr, Extension, Extensions};
use picky::{hash::HashAlgorithm, oids, signature::SignatureAlgorithm};
use std::error::Error;

// Generate a self-signed root certificate
fn main() -> Result<(), Box<dyn Error>> {
    let (root_key, _) =
        gen_rsa_key_pem_and_file("certs/nodns/root_pri.key", "certs/nodns/root_pub.key").unwrap();
    let (intermediate_pri, _) = gen_rsa_key_pem_and_file(
        "certs/nodns/intermediate_pri.key",
        "certs/nodns/intermediate_pub.key",
    )
    .unwrap();
    let (leaf_key, _) =
        gen_rsa_key_pem_and_file("certs/nodns/jmhuang_pri.key", "certs/nodns/jmhuang_pub.key")
            .unwrap();

    let (localhost_key, _) = gen_rsa_key_pem_and_file(
        "certs/nodns/localhost_pri.key",
        "certs/nodns/localhost_pub.key",
    )
    .unwrap();

    let (from_date, to_date) = gen_valid_date(3)?;
    let root = gen_root_cert(
        "MyRootCa",
        from_date,
        to_date,
        &root_key,
        "certs/nodns/root.crt",
    )?;
    assert_eq!(root.ty(), CertType::Root);
    let (from_date, to_date) = gen_valid_date(3)?;
    let intermediate = gen_ca_cert(
        "MyIntermediateCa",
        from_date,
        to_date,
        &root,
        &root_key,
        &intermediate_pri,
        "certs/nodns/intermediate.crt",
    )?;
    assert_eq!(intermediate.ty(), CertType::Intermediate);

    // Generate leaf certificate signed by intermediate authority

    let mut key_usage = KeyUsage::new(3);
    key_usage.set_digital_signature(false);
    key_usage.set_content_commitment(false);
    key_usage.set_key_encipherment(false);
    let extensions = Extensions(vec![
        Extension::new_basic_constraints(None, None).into_non_critical(),
        Extension::new_key_usage(key_usage).into_non_critical(),
        Extension::new_extended_key_usage(vec![
            oids::kp_client_auth(),
            oids::kp_server_auth(),
            oids::kp_code_signing(),
        ])
        .into_non_critical(),
        // Extension::new_subject_alt_name(vec![
        //     GeneralName::new_dns_name("www.localhost.com")
        //         .unwrap()
        //         .into(),
        //     GeneralName::new_dns_name("localhost.com").unwrap().into(),
        // ])
        // .into_non_critical(),
    ]);
    let attr = Attribute::new_extension_request(extensions.0);
    let mut my_name = DirectoryName::new_common_name("jmhuang");
    my_name.add_attr(NameAttr::StateOrProvinceName, "fujian");
    my_name.add_attr(NameAttr::CountryName, "China");
    let csr = Csr::generate_with_attributes(
        my_name,
        &leaf_key,
        SignatureAlgorithm::RsaPkcs1v15(HashAlgorithm::SHA2_256),
        vec![attr],
    )
    .unwrap();
    let (from_date, to_date) = gen_valid_date(3)?;

    let signed_leaf = gen_cert_by_ca(
        csr,
        from_date,
        to_date,
        &intermediate,
        &intermediate_pri,
        "certs/nodns/jmhuang.crt",
    )
    .unwrap();

    assert_eq!(signed_leaf.ty(), CertType::Leaf);

    let mut key_usage = KeyUsage::new(3);
    key_usage.set_digital_signature(false);
    key_usage.set_content_commitment(false);
    key_usage.set_key_encipherment(false);
    let extensions = Extensions(vec![
        Extension::new_basic_constraints(None, None).into_non_critical(),
        Extension::new_key_usage(key_usage).into_non_critical(),
        Extension::new_extended_key_usage(vec![
            oids::kp_client_auth(),
            oids::kp_server_auth(),
            oids::kp_code_signing(),
        ])
        .into_non_critical(),
        // Extension::new_subject_alt_name(vec![
        //     GeneralName::new_dns_name("www.localhost.com")
        //         .unwrap()
        //         .into(),
        //     GeneralName::new_dns_name("localhost.com").unwrap().into(),
        // ])
        // .into_non_critical(),
    ]);
    let attr = Attribute::new_extension_request(extensions.0);
    let mut my_name = DirectoryName::new_common_name("localhost");
    my_name.add_attr(NameAttr::StateOrProvinceName, "fujian");
    my_name.add_attr(NameAttr::CountryName, "China");
    let csr = Csr::generate_with_attributes(
        my_name,
        &localhost_key,
        SignatureAlgorithm::RsaPkcs1v15(HashAlgorithm::SHA2_256),
        vec![attr],
    )
    .unwrap();
    let (from_date, to_date) = gen_valid_date(3)?;

    let signed_leaf = gen_cert_by_ca(
        csr,
        from_date,
        to_date,
        &intermediate,
        &intermediate_pri,
        "certs/nodns/localhost.crt",
    )
    .unwrap();

    // Check leaf using CA chain

    let chain = [intermediate, root];
    signed_leaf
        .verifier()
        .chain(chain.iter())
        .exact_date(&UTCDate::ymd(2022, 12, 20).unwrap())
        .verify()?;
    let err = signed_leaf
        .verifier()
        .chain(chain.iter())
        .exact_date(&UTCDate::ymd(2025, 12, 2).unwrap())
        .verify()
        .err()
        .unwrap();
    println!("{:?}", err.to_string());
    // assert_eq!(
    //     err.to_string(),
    //     "invalid certificate \'CN=jmhuang,ST=fujian,C=China\': \
    //  certificate expired (not after: 2024-01-01 00:00:00, now: 2025-01-02 00:00:00)"
    // );
    Ok(())
}
