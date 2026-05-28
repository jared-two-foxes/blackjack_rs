use blackjack::data_source::{DataSource, DEFAULT_TABLE_COUNT};

#[test]
fn test_generate_tables_creates_expected_number() {
    let mut ds = DataSource::default();
    let ids = ds.generate_tables(DEFAULT_TABLE_COUNT);
    assert_eq!(ids.len(), DEFAULT_TABLE_COUNT);
    // Each table should have a unique id and be present in the hands as a dealer
    for id in &ids {
        assert!(ds.hands.iter().any(|h| h.dealer == *id && h.player == *id));
    }
}

#[test]
fn test_generate_tables_custom_count() {
    let mut ds = DataSource::default();
    let count = 10;
    let ids = ds.generate_tables(count);
    assert_eq!(ids.len(), count);
    for id in &ids {
        assert!(ds.hands.iter().any(|h| h.dealer == *id && h.player == *id));
    }
}
