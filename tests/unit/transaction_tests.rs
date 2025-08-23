// use speed_blockchain::Transaction;

// #[test]
// fn test_transaction_creation() {
//     let tx = Transaction::new("alice".to_string(), "bob".to_string(), 100, 5);

//     // Test basic fields
//     assert_eq!(tx.from, "alice");
//     assert_eq!(tx.to, "bob");
//     assert_eq!(tx.amount, 100);
//     assert_eq!(tx.fee, 5);

//     // Test auto-generated fields
//     assert!(tx.timestamp > 0);
//     assert_eq!(tx.nonce, 0); // Default nonce
//     assert!(!tx.signature.is_empty());

//     println!("✅ Transaction created successfully!");
//     println!("   From: {} -> To: {}", tx.from, tx.to);
//     println!("   Amount: {}, Fee: {}", tx.amount, tx.fee);
//     println!("   Timestamp: {}", tx.timestamp);
//     println!("   Signature: {}", tx.signature);
// }

// #[test]
// fn test_transaction_signature_format() {
//     let tx = Transaction::new("alice".to_string(), "bob".to_string(), 100, 5);

//     // Test signature format: "sig_{from}_{to}_{amount}_{fee}"
//     let expected_signature = "sig_alice_bob_100_5";
//     assert_eq!(tx.signature, expected_signature);

//     println!("✅ Signature format is correct: {}", tx.signature);
// }

// #[test]
// fn test_transaction_with_different_values() {
//     let tx = Transaction::new("user123".to_string(), "user456".to_string(), 999, 10);

//     assert_eq!(tx.from, "user123");
//     assert_eq!(tx.to, "user456");
//     assert_eq!(tx.amount, 999);
//     assert_eq!(tx.fee, 10);
//     assert_eq!(tx.signature, "sig_user123_user456_999_10");

//     println!("✅ Transaction with different values works!");
// }

// #[test]
// fn test_transaction_timestamp_is_recent() {
//     let tx = Transaction::new("alice".to_string(), "bob".to_string(), 100, 5);

//     // Get current timestamp
//     let now = std::time::SystemTime::now()
//         .duration_since(std::time::UNIX_EPOCH)
//         .unwrap()
//         .as_secs();

//     // Transaction timestamp should be very recent (within 5 seconds)
//     assert!(tx.timestamp <= now);
//     assert!(tx.timestamp > now - 5);

//     println!("✅ Timestamp is recent: {} (now: {})", tx.timestamp, now);
// }

// #[test]
// fn test_transaction_nonce_default() {
//     let tx = Transaction::new("alice".to_string(), "bob".to_string(), 100, 5);

//     // Nonce should default to 0
//     assert_eq!(tx.nonce, 0);

//     println!("✅ Default nonce is 0");
// }

// #[test]
// fn test_transaction_zero_amounts() {
//     let tx = Transaction::new(
//         "alice".to_string(),
//         "bob".to_string(),
//         0, // Zero amount
//         0, // Zero fee
//     );

//     assert_eq!(tx.amount, 0);
//     assert_eq!(tx.fee, 0);
//     assert_eq!(tx.signature, "sig_alice_bob_0_0");

//     println!("✅ Zero amounts work correctly");
// }

// #[test]
// fn test_transaction_large_amounts() {
//     let large_amount = 1_000_000_000u64; // 1 billion
//     let tx = Transaction::new(
//         "whale".to_string(),
//         "exchange".to_string(),
//         large_amount,
//         1000,
//     );

//     assert_eq!(tx.amount, large_amount);
//     assert_eq!(tx.fee, 1000);

//     println!("✅ Large amounts work: {}", large_amount);
// }

// #[test]
// fn test_transaction_empty_addresses() {
//     let tx = Transaction::new(
//         "".to_string(), // Empty from
//         "".to_string(), // Empty to
//         100,
//         5,
//     );

//     assert_eq!(tx.from, "");
//     assert_eq!(tx.to, "");
//     assert_eq!(tx.signature, "sig___100_5");

//     println!("✅ Empty addresses are handled");
// }

// #[test]
// fn test_transaction_special_characters() {
//     let tx = Transaction::new(
//         "alice@domain.com".to_string(),
//         "bob_123".to_string(),
//         100,
//         5,
//     );

//     assert_eq!(tx.from, "alice@domain.com");
//     assert_eq!(tx.to, "bob_123");
//     assert_eq!(tx.signature, "sig_alice@domain.com_bob_123_100_5");
// }
