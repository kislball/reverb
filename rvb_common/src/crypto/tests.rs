use super::*;

#[test]
fn test_import_armored_private_and_public() {
    let keypair = KeyPair::generate();
    let armored_private = keypair.armor_private();
    let armored_public = keypair.armor_public();

    let imported_private = KeyPair::import_armored(&armored_private).unwrap();
    assert_eq!(keypair.export_private(), imported_private.export_private());

    let imported_public = PublicKey::import_armored(&armored_public).unwrap();
    assert_eq!(keypair.public().export(), imported_public.export());
}

#[test]
fn test_import_armored_invalid_data() {
    // Not base64
    assert!(KeyPair::import_armored("not_base64!").is_err());
    assert!(PublicKey::import_armored("not_base64!").is_err());

    // Base64 but wrong length
    let invalid = base64::engine::general_purpose::STANDARD.encode([1, 2, 3]);
    assert!(KeyPair::import_armored(&invalid).is_err());
    assert!(PublicKey::import_armored(&invalid).is_err());
}

#[test]
fn test_import_armored_roundtrip() {
    let keypair = KeyPair::generate();
    let armored = keypair.armor_private();
    let imported = KeyPair::import_armored(&armored).unwrap();
    assert_eq!(keypair.export_private(), imported.export_private());

    let public = keypair.public();
    let armored_pub = public.armor();
    let imported_pub = PublicKey::import_armored(&armored_pub).unwrap();
    assert_eq!(public.export(), imported_pub.export());
}

#[test]
fn test_keypair_generate_and_export_import() {
    let keypair = KeyPair::generate();
    let exported = keypair.export_private();
    let imported = KeyPair::import(&exported).unwrap();
    assert_eq!(keypair.export_private(), imported.export_private());
}

#[test]
fn test_public_key_export_import() {
    let keypair = KeyPair::generate();
    let public = keypair.public();
    let exported = public.export();
    let imported = PublicKey::import(&exported).unwrap();
    assert_eq!(public.export(), imported.export());
}

#[test]
fn test_armor_private_and_public() {
    let keypair = KeyPair::generate();
    let armor_private = keypair.armor_private();
    let armor_public = keypair.armor_public();
    assert_eq!(armor_private, b64_encode(&keypair.export_private()));
    assert_eq!(armor_public, b64_encode(&keypair.export_public()));
}

#[test]
fn test_armor_public_key() {
    let keypair = KeyPair::generate();
    let public = keypair.public();
    let armor = public.armor();
    assert_eq!(armor, b64_encode(&public.export()));
}

#[test]
fn test_sign_and_verify() {
    let mut keypair = KeyPair::generate();
    let data = b"hello world";
    let signature = keypair.sign(data);
    assert!(keypair.verify(data, &signature));
}

#[test]
fn test_public_key_verify() {
    let mut keypair = KeyPair::generate();
    let public = keypair.public();
    let data = b"test message";
    let signature = keypair.sign(data);
    assert!(public.verify(data, &signature));
    assert!(!public.verify(b"other", &signature));
}

#[test]
fn test_encrypt_decrypt() {
    let keypair = KeyPair::generate();
    let data = b"secret data";
    let encrypted = keypair.encrypt(data).unwrap();
    let decrypted = keypair.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_public_key_encrypt() {
    let keypair = KeyPair::generate();
    let public = keypair.public();
    let data = b"public encrypt";
    let encrypted = public.encrypt(data).unwrap();
    let decrypted = keypair.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_import_invalid_length() {
    assert!(KeyPair::import(&[0u8; 10]).is_err());
    assert!(PublicKey::import(&[0u8; 10]).is_err());
}

#[test]
fn test_export_public() {
    let keypair = KeyPair::generate();
    let exported = keypair.export_public();
    assert_eq!(exported.len(), 64);
}

#[test]
fn test_invalid_signature_verification() {
    let keypair = KeyPair::generate();
    let public = keypair.public();
    let data = b"data";
    let invalid_signature = vec![0u8; 64];
    assert!(!public.verify(data, &invalid_signature));
    assert!(!keypair.verify(data, &invalid_signature));
}
