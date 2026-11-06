//! The feature table (features.json, read by CMake, cargo and `bui build`):
//! blenderui forwards each optional part to blenderui-sys under its name.

#[test]
fn every_kit_feature_is_a_cargo_feature() {
    let root = env!("CARGO_MANIFEST_DIR");
    let table = std::fs::read_to_string(format!("{root}/../../../features.json")).unwrap();
    let manifest = std::fs::read_to_string(format!("{root}/Cargo.toml")).unwrap();
    let names = table.lines().filter(|l| l.contains("\"option\": \"")).filter_map(|l| {
        let at = l.find("\"name\": \"")? + 9;
        l[at..].split('"').next()
    });
    let mut n = 0;
    for name in names.filter(|&n| n != "imgui") {
        let row = format!("{name} = [\"blenderui-sys/{name}\"]");
        assert!(manifest.lines().any(|l| l.trim() == row), "blenderui's Cargo.toml lacks `{row}`");
        n += 1;
    }
    assert!(n >= 10, "read the table ({n} features)");
}
